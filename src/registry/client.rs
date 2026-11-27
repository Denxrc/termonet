//! Network transport abstraction and bounded HTTPS client for registry fetches.

use crate::error::{Result, TermonetError};
use std::io::Read;
use std::time::Duration;

/// Maximum payload size for `registry.toml` (2 MiB).
pub const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
/// Maximum payload size for individual rule/template files (512 KiB).
pub const MAX_ASSET_BYTES: usize = 512 * 1024;

/// Abstraction over HTTP text fetching to enable deterministic offline tests.
pub trait RegistryTransport: Send + Sync {
    /// Fetches UTF-8 text from `url`, enforcing a strict maximum byte limit.
    fn fetch_text(&self, url: &str, max_bytes: usize) -> Result<String>;
}

/// Production synchronous HTTPS client powered by `ureq` + `rustls`.
#[derive(Debug, Clone)]
pub struct HttpTransport {
    https_agent: ureq::Agent,
    local_agent: ureq::Agent,
}

impl HttpTransport {
    /// Creates a new transport with 5s connect / 10s read timeouts.
    #[must_use]
    pub fn new() -> Self {
        let https_agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(10))
            .https_only(true)
            .redirects(3)
            .user_agent(concat!("termonet/", env!("CARGO_PKG_VERSION")))
            .build();

        let local_agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(10))
            .redirects(0)
            .user_agent(concat!("termonet/", env!("CARGO_PKG_VERSION")))
            .build();

        Self {
            https_agent,
            local_agent,
        }
    }
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

fn is_allowed_local_http(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split(&['/', '?', '#'][..]).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    if let Some(bracketed) = authority.strip_prefix("[::1]") {
        bracketed.is_empty()
            || bracketed
                .strip_prefix(':')
                .is_some_and(|port| !port.is_empty() && port.parse::<u16>().is_ok())
    } else if let Some((host, port)) = authority.split_once(':') {
        (host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1")
            && !port.is_empty()
            && port.parse::<u16>().is_ok()
    } else {
        authority.eq_ignore_ascii_case("localhost") || authority == "127.0.0.1"
    }
}

impl RegistryTransport for HttpTransport {
    fn fetch_text(&self, url: &str, max_bytes: usize) -> Result<String> {
        let trimmed = url.trim();
        let agent = if trimmed.starts_with("https://") {
            &self.https_agent
        } else if is_allowed_local_http(trimmed) {
            &self.local_agent
        } else {
            return Err(TermonetError::Registry(format!(
                "Only HTTPS repository URLs are allowed (rejected '{trimmed}')"
            )));
        };

        let resp = agent.get(trimmed).call().map_err(|err| {
            TermonetError::Registry(format!("HTTP request failed for '{trimmed}': {err}"))
        })?;

        if let Some(len_str) = resp.header("Content-Length")
            && let Ok(len) = len_str.trim().parse::<usize>()
            && len > max_bytes
        {
            return Err(TermonetError::Registry(format!(
                "Response from '{trimmed}' ({len} bytes) exceeds maximum allowed size ({max_bytes} bytes)"
            )));
        }

        let mut buf = Vec::new();
        let limit = (max_bytes as u64).saturating_add(1);
        resp.into_reader()
            .take(limit)
            .read_to_end(&mut buf)
            .map_err(|err| {
                TermonetError::Registry(format!("Failed reading body from '{trimmed}': {err}"))
            })?;

        if buf.len() > max_bytes {
            return Err(TermonetError::Registry(format!(
                "Response from '{trimmed}' exceeds maximum allowed size ({max_bytes} bytes)"
            )));
        }

        String::from_utf8(buf).map_err(|err| {
            TermonetError::Registry(format!("Invalid UTF-8 payload from '{trimmed}': {err}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};

    #[test]
    fn test_http_transport_rejects_insecure_url() {
        let transport = HttpTransport::new();
        let res = transport.fetch_text("http://example.com/registry.toml", MAX_MANIFEST_BYTES);
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(err.contains("HTTPS"), "Unexpected error: {err}");
    }

    #[test]
    fn test_http_transport_rejects_oversized_payload() {
        let transport = HttpTransport::new();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test listener");
        let addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0u8; 512];
                let _ = stream.read(&mut req_buf);
                let body = "A".repeat(256);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let url = format!("http://{}/registry.toml", addr);
        let oversized = transport.fetch_text(&url, 128);
        let _ = TcpStream::connect_timeout(&addr, Duration::from_millis(100));
        let _ = handle.join();

        assert!(oversized.is_err());
        let err_msg = oversized.unwrap_err().to_string();
        assert!(
            err_msg.contains("exceeds maximum allowed size"),
            "Unexpected error message: {err_msg}"
        );
    }

    #[test]
    fn test_http_transport_reads_valid_payload() {
        let transport = HttpTransport::new();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test listener");
        let addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0u8; 512];
                let _ = stream.read(&mut req_buf);
                let body = "schema_version = 1\n";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let url = format!("http://{}/registry.toml", addr);
        let body = transport.fetch_text(&url, 1024).unwrap();
        let _ = handle.join();
        assert_eq!(body, "schema_version = 1\n");
    }

    #[test]
    fn test_stress_http_transport_userinfo_bypass_and_non_utf8() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = std::thread::spawn(move || {
            for idx in 0..2 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = [0u8; 512];
                    let _ = stream.read(&mut buf);
                    if idx == 0 {
                        let _ = stream.write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nhello",
                        );
                    } else {
                        let mut resp =
                            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\n"
                                .to_vec();
                        resp.extend_from_slice(&[0xff, 0xfe, 0xfd, 0xfc]);
                        let _ = stream.write_all(&resp);
                    }
                }
            }
        });

        let transport = HttpTransport::new();

        // Userinfo trick http://localhost:@127.0.0.1:<port> must be rejected as non-localhost authority
        let userinfo_url = format!("http://localhost:@127.0.0.1:{}/registry.toml", addr.port());
        let bypass_res = transport.fetch_text(&userinfo_url, MAX_MANIFEST_BYTES);
        assert!(
            bypass_res.is_err(),
            "HttpTransport must reject userinfo SSRF bypass URL: {bypass_res:?}"
        );

        // Non-UTF-8 payload from localhost must error cleanly
        let bad_utf8_url = format!("http://{addr}/bad_utf8");
        assert!(
            transport
                .fetch_text(&bad_utf8_url, MAX_ASSET_BYTES)
                .is_err()
        );

        let _ = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(50));
        let _ = server.join();
    }
}
