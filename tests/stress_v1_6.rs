use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use termonet::config::AppConfig;
use termonet::error::{Result, TermonetError};
use termonet::registry::{
    AutoUpdateOutcome, HttpTransport, MAX_ASSET_BYTES, MAX_MANIFEST_BYTES, OFFICIAL_REPO_URL,
    RegistryPaths, RegistryService, RegistryTransport, is_version_compatible,
    sanitize_relative_repo_path, sha256_hex, validate_program_id,
};
use termonet::tui::app::{AppState, Step, WizardStep};
use termonet::tui::views;

static DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(label: &str) -> Self {
        let id = DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "termonet_stress_{}_{}_{}",
            label,
            std::process::id(),
            id
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp test dir");
        Self { path }
    }

    fn registry_paths(&self) -> RegistryPaths {
        RegistryPaths {
            repos_dir: self.path.join("repos"),
            rules_dir: self.path.join("rules"),
            templates_dir: self.path.join("templates"),
            cache_dir: self.path.join("cache"),
            installed_file: self.path.join("installed.toml"),
            state_file: self.path.join("state.toml"),
        }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Clone)]
struct MockTransport {
    responses: Arc<Mutex<HashMap<String, std::result::Result<String, String>>>>,
    delay: Option<Duration>,
}

impl MockTransport {
    fn new(responses: HashMap<String, std::result::Result<String, String>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            delay: None,
        }
    }
}

impl RegistryTransport for MockTransport {
    fn fetch_text(&self, url: &str, max_bytes: usize) -> Result<String> {
        if let Some(d) = self.delay {
            thread::sleep(d);
        }
        let guard = self.responses.lock().expect("lock mock responses");
        match guard.get(url) {
            Some(Ok(body)) if body.len() <= max_bytes => Ok(body.clone()),
            Some(Ok(_)) => Err(TermonetError::Registry("exceeds max_bytes".into())),
            Some(Err(msg)) => Err(TermonetError::Registry(msg.clone())),
            None => Err(TermonetError::Registry(format!("404 mock: {url}"))),
        }
    }
}

fn sample_rule(name: &str, version: u32, template: &str) -> String {
    format!(
        "version = {version}\nname = \"{name}\"\ntemplate = \"{template}\"\noutput_path = \"~/.config/app/colors.conf\"\n"
    )
}

fn sample_tmpl(version: u32) -> String {
    format!("# termonet-version: {version}\nfg = {{{{ active.primary.hex }}}}\n")
}

#[allow(clippy::too_many_arguments)]
fn manifest_entry(
    id: &str,
    name: &str,
    rule_rel: &str,
    tmpl_rel: &str,
    rv: u32,
    tv: u32,
    min_ver: &str,
    r_sha: &str,
    t_sha: &str,
) -> String {
    format!(
        "[[programs]]\nid = \"{id}\"\nname = \"{name}\"\ncategory = \"terminal\"\ndescription = \"{name}\"\ndescription_ru = \"{name}\"\nrule = \"{rule_rel}\"\ntemplate = \"{tmpl_rel}\"\nrule_version = {rv}\ntemplate_version = {tv}\nmin_termonet_version = \"{min_ver}\"\nrule_sha256 = \"{r_sha}\"\ntemplate_sha256 = \"{t_sha}\"\n\n"
    )
}

// ============================================================================
// AXIS A & B: Registry manifests, repos/*.toml, path traversal, semver, sizes
// ============================================================================

#[test]
fn stress_axis_ab_empty_corrupted_and_oversized_registry_and_repos() {
    let tmp = TempDirGuard::new("reg_ab");
    let paths = tmp.registry_paths();
    fs::create_dir_all(&paths.repos_dir).unwrap();

    // 1. Various corrupted/edge repos/*.toml
    fs::write(paths.repos_dir.join("empty.toml"), "").unwrap();
    fs::write(paths.repos_dir.join("nullbytes.toml"), "\0\0\0\0").unwrap();
    fs::write(
        paths.repos_dir.join("invalid_utf8.toml"),
        [0xff, 0xfe, 0xfd],
    )
    .unwrap();
    fs::write(
        paths.repos_dir.join("traversal_name.toml"),
        "name = \"../evil\"\nurl = \"https://example.com\"\n",
    )
    .unwrap();
    fs::write(
        paths.repos_dir.join("empty_url.toml"),
        "name = \"emptyurl\"\nurl = \"   \"\n",
    )
    .unwrap();
    fs::write(
        paths.repos_dir.join("huge_garbage.toml"),
        "a = 1\n".repeat(100_000),
    )
    .unwrap();
    // Directory masquerading as .toml
    fs::create_dir_all(paths.repos_dir.join("dir_as_file.toml")).unwrap();

    // Valid official repo
    fs::write(
        paths.repos_dir.join("official.toml"),
        format!(
            "name = \"official\"\nurl = \"{OFFICIAL_REPO_URL}\"\npriority = 0\nenabled = true\n"
        ),
    )
    .unwrap();

    let mut responses = HashMap::new();
    // Corrupted registry.toml cases
    let bad_manifests = [
        "not toml ][",
        "schema_version = 999\n",
        "schema_version = 0\n",
        "schema_version = 1\n[[programs]]\nid = \"../escape\"\nname = \"Bad\"\nrule = \"rules/a.toml\"\ntemplate = \"templates/a.tmpl\"\nrule_version = 1\ntemplate_version = 1\nmin_termonet_version = \"1.6.0\"\nrule_sha256 = \"aa\"\ntemplate_sha256 = \"bb\"\n",
        "schema_version = 1\n[[programs]]\nid = \"good\"\nname = \"Good\"\nrule = \"../secret.toml\"\ntemplate = \"templates/a.tmpl\"\nrule_version = 1\ntemplate_version = 1\nmin_termonet_version = \"1.6.0\"\nrule_sha256 = \"aa\"\ntemplate_sha256 = \"bb\"\n",
        "schema_version = 1\n[[programs]]\nid = \"good\"\nname = \"Good\"\nrule = \"rules/a.toml\"\ntemplate = \"templates/../../etc/passwd\"\nrule_version = 1\ntemplate_version = 1\nmin_termonet_version = \"1.6.0\"\nrule_sha256 = \"aa\"\ntemplate_sha256 = \"bb\"\n",
        // Duplicate IDs
        "schema_version = 1\n[[programs]]\nid = \"dup\"\nname = \"A\"\nrule = \"rules/a.toml\"\ntemplate = \"templates/a.tmpl\"\nrule_version = 1\ntemplate_version = 1\nmin_termonet_version = \"1.6.0\"\nrule_sha256 = \"aa\"\ntemplate_sha256 = \"bb\"\n[[programs]]\nid = \"dup\"\nname = \"B\"\nrule = \"rules/b.toml\"\ntemplate = \"templates/b.tmpl\"\nrule_version = 1\ntemplate_version = 1\nmin_termonet_version = \"1.6.0\"\nrule_sha256 = \"aa\"\ntemplate_sha256 = \"bb\"\n",
    ];

    for bad in bad_manifests {
        responses.insert(
            format!("{OFFICIAL_REPO_URL}/registry.toml"),
            Ok(bad.to_string()),
        );
        let svc = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(responses.clone()),
            "1.6.0".into(),
        );
        let (repos, warnings) = svc.load_repos().unwrap();
        assert_eq!(repos.len(), 1);
        assert!(!warnings.is_empty());
        assert!(
            svc.sync_indexes().is_err(),
            "Bad manifest must be rejected:\n{bad}"
        );
    }

    // Empty registry.toml or unrelated keys must be rejected because schema_version is required.
    responses.insert(
        format!("{OFFICIAL_REPO_URL}/registry.toml"),
        Ok("unrelated_key = \"hello\"\n".to_string()),
    );
    let svc_empty = RegistryService::with_paths_and_transport(
        paths.clone(),
        MockTransport::new(responses.clone()),
        "1.6.0".into(),
    );
    let sync_empty_res = svc_empty.sync_indexes();
    assert!(
        sync_empty_res.is_err(),
        "Manifest without schema_version must be rejected: {sync_empty_res:?}"
    );

    // Giant manifest (~1.8 MiB, ~5000 programs) must parse & catalog quickly without panic
    let mut big_manifest = String::from("schema_version = 1\n\n");
    for i in 0..4500 {
        big_manifest.push_str(&manifest_entry(
            &format!("app-{i:04}"),
            &format!("App {i}"),
            &format!("rules/app-{i:04}.toml"),
            &format!("templates/app-{i:04}.tmpl"),
            1,
            1,
            "1.6.0",
            "0000",
            "0000",
        ));
    }
    assert!(big_manifest.len() < MAX_MANIFEST_BYTES);
    responses.insert(
        format!("{OFFICIAL_REPO_URL}/registry.toml"),
        Ok(big_manifest),
    );
    let svc_big = RegistryService::with_paths_and_transport(
        paths,
        MockTransport::new(responses),
        "1.6.0".into(),
    );
    let t0 = Instant::now();
    svc_big.sync_indexes().unwrap();
    let (catalog, _) = svc_big.catalog().unwrap();
    assert_eq!(catalog.len(), 4500);
    assert!(
        t0.elapsed() < Duration::from_secs(5),
        "4500-program manifest took too long: {:?}",
        t0.elapsed()
    );
}

#[test]
fn stress_axis_ab_path_traversal_and_semver_boundaries() {
    let long_64 = "a".repeat(64);
    let long_65 = "a".repeat(65);
    assert!(validate_program_id(&long_64).is_ok());
    assert!(validate_program_id(&long_65).is_err());

    for bad_id in [
        "",
        " ",
        "../kitty",
        "..\\kitty",
        "a/b",
        "a\\b",
        "a..b",
        "kitty\0",
        "kitty\n",
        "-kitty",
        "_kitty",
        "Kitty",
        "китти",
        "kitty.toml",
    ] {
        assert!(
            validate_program_id(bad_id).is_err(),
            "ID '{bad_id:?}' must be rejected"
        );
    }

    for bad_rel in [
        "",
        "/",
        "/etc/passwd",
        "..",
        ".",
        "../rules/kitty.toml",
        "rules/./kitty.toml",
        "rules/../kitty.toml",
        "rules//kitty.toml",
        "rules/kitty.toml/",
        "rules\\kitty.toml",
        "rules/..\\kitty.toml",
    ] {
        assert!(
            sanitize_relative_repo_path(bad_rel).is_err(),
            "Path '{bad_rel:?}' must be rejected"
        );
    }

    // Semver edge cases
    assert!(is_version_compatible("0.0.0", "0.0.0").unwrap());
    assert!(is_version_compatible("1.6.0", "v1.6.0").unwrap());
    assert!(is_version_compatible("v1.6.0", "1.6.1").unwrap());
    assert!(!is_version_compatible("1.6.1", "1.6.0").unwrap());
    assert!(!is_version_compatible("2.0.0", "1.999.999").unwrap());
    assert!(
        is_version_compatible(&format!("{}.0.0", u64::MAX), &format!("{}.0.0", u64::MAX)).unwrap()
    );

    for bad_ver in [
        "",
        "1.6",
        "1.6.0.0",
        "1.6.0-alpha",
        "1.6.0+build",
        "-1.0.0",
        "1.a.0",
        "999999999999999999999999.0.0",
    ] {
        assert!(
            is_version_compatible(bad_ver, "1.6.0").is_err(),
            "Invalid semver '{bad_ver}' must error"
        );
    }
}

#[test]
fn stress_axis_c_http_transport_userinfo_bypass_and_truncated_stream() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        // Handle up to 2 connections without hanging
        for idx in 0..2 {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 512];
                let _ = stream.read(&mut buf);
                if idx == 0 {
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nhello",
                    );
                } else {
                    // Non-UTF-8 payload
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

    // 1. Check userinfo trick: http://localhost:@127.0.0.1:<port>/registry.toml
    // MUST be rejected as non-localhost authority (actual host is 127.0.0.1 here, or evil.com in real attack)!
    let userinfo_url = format!("http://localhost:@127.0.0.1:{}/registry.toml", addr.port());
    let bypass_res = transport.fetch_text(&userinfo_url, MAX_MANIFEST_BYTES);
    assert!(
        bypass_res.is_err(),
        "HttpTransport must reject userinfo SSRF bypass URL: {bypass_res:?}"
    );

    // 2. Non-UTF-8 payload from localhost must error cleanly
    let bad_utf8_url = format!("http://{addr}/bad_utf8");
    let res_utf8 = transport.fetch_text(&bad_utf8_url, MAX_ASSET_BYTES);
    assert!(res_utf8.is_err());

    let _ = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(50));
    let _ = server.join();
}

#[test]
fn stress_axis_ab_tampered_installed_toml_path_traversal() {
    let tmp = TempDirGuard::new("tampered_installed");
    let paths = tmp.registry_paths();
    fs::create_dir_all(&paths.rules_dir).unwrap();
    fs::create_dir_all(&paths.templates_dir).unwrap();

    // Victim file outside rules_dir/templates_dir
    let victim_rule = tmp.path.join("victim.toml");
    let victim_tmpl = tmp.path.join("victim.tmpl");
    fs::write(&victim_rule, "important user file").unwrap();
    fs::write(&victim_tmpl, "important template").unwrap();

    // Tampered installed.toml with relative traversal
    let installed_toml = r#"
schema_version = 1

[[installed]]
id = "kitty"
repo = "official"
rule_file = "../victim.toml"
template_file = "../victim.tmpl"
rule_version = 1
template_version = 1
rule_sha256 = "00"
template_sha256 = "00"
"#;
    fs::write(&paths.installed_file, installed_toml).unwrap();

    let svc = RegistryService::with_paths_and_transport(
        paths,
        MockTransport::new(HashMap::new()),
        "1.6.0".into(),
    );
    let rm_res = svc.remove_program("kitty");
    assert!(
        rm_res.is_err(),
        "remove_program must reject path-traversal filenames in installed.toml: {rm_res:?}"
    );
    assert!(victim_rule.exists());
    assert!(victim_tmpl.exists());
}

#[test]
fn stress_axis_bc_update_rules_modified_same_version_and_reset_backups() {
    let tmp = TempDirGuard::new("update_vs_reset");
    let paths = tmp.registry_paths();
    let r1 = sample_rule("Foot", 1, "foot.tmpl");
    let t1 = sample_tmpl(1);
    let m1 = format!(
        "schema_version = 1\n\n{}",
        manifest_entry(
            "foot",
            "Foot",
            "rules/foot.toml",
            "templates/foot.tmpl",
            1,
            1,
            "1.6.0",
            &sha256_hex(r1.as_bytes()),
            &sha256_hex(t1.as_bytes()),
        )
    );
    let mut map = HashMap::new();
    map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(m1));
    map.insert(
        format!("{OFFICIAL_REPO_URL}/rules/foot.toml"),
        Ok(r1.clone()),
    );
    map.insert(
        format!("{OFFICIAL_REPO_URL}/templates/foot.tmpl"),
        Ok(t1.clone()),
    );

    let svc = RegistryService::with_paths_and_transport(
        paths.clone(),
        MockTransport::new(map),
        "1.6.0".into(),
    );
    svc.ensure_default_repo_config().unwrap();
    svc.sync_indexes().unwrap();
    svc.install_programs(&["foot".into()], false, false)
        .unwrap();

    // User deletes local template (or modifies it) while upstream version is still 1/1
    fs::write(paths.templates_dir.join("foot.tmpl"), "USER EDITED\n").unwrap();

    let rep_no_force = svc.update_installed(false, false).unwrap();
    assert_eq!(rep_no_force.skipped_modified, vec!["foot".to_string()]);
    assert!(rep_no_force.updated.is_empty());

    // Forced reset with create_backups=true creates .bak
    let rep_force_bak = svc.update_installed(true, true).unwrap();
    assert_eq!(rep_force_bak.updated, vec!["foot".to_string()]);
    assert!(paths.templates_dir.join("foot.tmpl.bak").exists());
    assert_eq!(
        fs::read_to_string(paths.templates_dir.join("foot.tmpl")).unwrap(),
        t1
    );
}

#[test]
fn stress_axis_c_commit_install_partial_failure_on_manifest_save() {
    let tmp = TempDirGuard::new("commit_manifest_fail");
    let paths = tmp.registry_paths();
    let r1 = sample_rule("Foot", 1, "foot.tmpl");
    let t1 = sample_tmpl(1);
    let m1 = format!(
        "schema_version = 1\n\n{}",
        manifest_entry(
            "foot",
            "Foot",
            "rules/foot.toml",
            "templates/foot.tmpl",
            1,
            1,
            "1.6.0",
            &sha256_hex(r1.as_bytes()),
            &sha256_hex(t1.as_bytes()),
        )
    );
    let mut map = HashMap::new();
    map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(m1));
    map.insert(format!("{OFFICIAL_REPO_URL}/rules/foot.toml"), Ok(r1));
    map.insert(format!("{OFFICIAL_REPO_URL}/templates/foot.tmpl"), Ok(t1));

    // Make installed.toml a directory so save_installed_manifest fails!
    fs::create_dir_all(&paths.installed_file).unwrap();

    let svc = RegistryService::with_paths_and_transport(
        paths.clone(),
        MockTransport::new(map),
        "1.6.0".into(),
    );
    svc.ensure_default_repo_config().unwrap();
    svc.sync_indexes().unwrap();

    let res = svc.install_programs(&["foot".into()], false, false);
    assert!(res.is_err());
    assert!(!paths.rules_dir.join("foot.toml").exists());
    assert!(!paths.templates_dir.join("foot.tmpl").exists());
}

// ============================================================================
// AXIS D: Concurrency & race conditions (16-32 threads)
// ============================================================================

#[test]
fn stress_axis_d_concurrent_sync_catalog_and_install() {
    let tmp = TempDirGuard::new("concurrency");
    let paths = tmp.registry_paths();

    let mut map = HashMap::new();
    let mut manifest = String::from("schema_version = 1\n\n");
    for i in 0..24 {
        let id = format!("app{i:02}");
        let r = sample_rule(&format!("App{i:02}"), 1, &format!("{id}.tmpl"));
        let t = sample_tmpl(1);
        manifest.push_str(&manifest_entry(
            &id,
            &format!("App{i:02}"),
            &format!("rules/{id}.toml"),
            &format!("templates/{id}.tmpl"),
            1,
            1,
            "1.6.0",
            &sha256_hex(r.as_bytes()),
            &sha256_hex(t.as_bytes()),
        ));
        map.insert(format!("{OFFICIAL_REPO_URL}/rules/{id}.toml"), Ok(r));
        map.insert(format!("{OFFICIAL_REPO_URL}/templates/{id}.tmpl"), Ok(t));
    }
    map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(manifest));

    let svc = Arc::new(RegistryService::with_paths_and_transport(
        paths.clone(),
        MockTransport::new(map),
        "1.6.0".into(),
    ));
    svc.ensure_default_repo_config().unwrap();
    svc.sync_indexes().unwrap();

    let mut handles = Vec::new();
    for i in 0..24 {
        let svc_c = Arc::clone(&svc);
        handles.push(thread::spawn(move || {
            let id = format!("app{i:02}");
            let _ = svc_c.sync_indexes();
            let _ = svc_c.catalog();
            svc_c.install_programs(&[id], false, false)
        }));
    }

    let mut ok_count = 0usize;
    for h in handles {
        if h.join().expect("worker thread must not panic").is_ok() {
            ok_count += 1;
        }
    }
    assert_eq!(ok_count, 24);

    let installed_text = fs::read_to_string(&paths.installed_file).unwrap_or_default();
    let installed_manifest: termonet::registry::InstalledManifest =
        toml::from_str(&installed_text).unwrap_or_default();
    assert_eq!(installed_manifest.installed.len(), 24);
}

// ============================================================================
// CLI binary edge-case tests
// ============================================================================

#[test]
fn stress_cli_binary_invalid_flags_and_empty_install() {
    let tmp = TempDirGuard::new("cli_bin");
    let bin = env!("CARGO_BIN_EXE_termonet");

    let run_cli = |args: &[&str]| {
        Command::new(bin)
            .args(args)
            .env("HOME", &tmp.path)
            .env("XDG_CONFIG_HOME", tmp.path.join("cfg"))
            .env("XDG_STATE_HOME", tmp.path.join("state"))
            .env("XDG_CACHE_HOME", tmp.path.join("cache"))
            .output()
            .expect("spawn termonet binary")
    };

    // 1. --install "" --apply MUST fail, NOT run --apply!
    let out1 = run_cli(&["--install", "", "--apply"]);
    assert!(
        !out1.status.success(),
        "`--install \"\" --apply` must fail, status={}",
        out1.status
    );

    // 2. --install ",,," --apply MUST fail, NOT run --apply!
    let out2 = run_cli(&["--install", ",,,", "--apply"]);
    assert!(
        !out2.status.success(),
        "`--install \",,,\" --apply` must fail, status={}",
        out2.status
    );

    // 3. --install ",,," alone must fail immediately with non-zero status (never launch TUI)
    let home_path = tmp.path.clone();
    let mut child = Command::new(bin)
        .args(["--install", ",,,"])
        .env("HOME", &home_path)
        .env("XDG_CONFIG_HOME", home_path.join("cfg"))
        .env("XDG_STATE_HOME", home_path.join("state"))
        .env("XDG_CACHE_HOME", home_path.join("cache"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn termonet");
    thread::sleep(Duration::from_millis(500));
    match child.try_wait().unwrap() {
        Some(status) => assert!(!status.success(), "`--install \",,,\"` must exit non-zero"),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`--install \",,,\"` hung instead of exiting with CLI error");
        }
    }

    // 4. Invalid flag combinations must exit non-zero
    for bad_args in [
        &["--list-rules", "--sync-repos"][..],
        &["--install", "kitty", "--update-rules"][..],
        &["--reset-rules", "--remove-rule", "kitty"][..],
        &["--install", "kitty", "--theme", "dark"][..],
        &["--theme", "neon"][..],
        &["--color", "nothex", "--apply"][..],
        &["--remove-rule", "../etc/passwd", "--yes"][..],
    ] {
        let out = run_cli(bad_args);
        assert!(
            !out.status.success(),
            "Expected failure for args {bad_args:?}, got success"
        );
    }

    // 5. --help and --version must succeed
    assert!(run_cli(&["--help"]).status.success());
    assert!(run_cli(&["--version"]).status.success());
}

// ============================================================================
// TUI modals, WizardStep::Finish, Step::Programs, narrow screens
// ============================================================================

#[test]
fn stress_tui_narrow_terminal_and_modals_no_panic() {
    let mut state = AppState::new(AppConfig::default());
    state.current_step = Step::Programs;
    state.confirm_modal = Some(termonet::tui::app::ConfirmModalState {
        title: "Confirm Install".into(),
        warning_lines: vec![
            "Line 1: VERY LONG WARNING LINE THAT EXCEEDS TERMINAL WIDTH SIGNIFICANTLY 1234567890"
                .into(),
            "WARNING reload_cmd: killall -SIGUSR1 foot".into(),
        ],
        action: termonet::tui::app::ConfirmAction::RemoveProgram { id: "foot".into() },
        scroll: 0,
    });

    // Normal 100x30 works
    let mut term_ok = Terminal::new(TestBackend::new(100, 30)).unwrap();
    term_ok
        .draw(|f| views::render_programs(f, f.area(), &state))
        .unwrap();

    // Narrow/tiny terminals with confirm_modal open must render without panic
    for (w, h) in [(0, 0), (1, 1), (10, 5), (30, 10)] {
        let mut term_narrow = Terminal::new(TestBackend::new(w, h)).unwrap();
        let narrow_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = term_narrow.draw(|f| views::render_programs(f, f.area(), &state));
        }));
        assert!(
            narrow_res.is_ok(),
            "render_confirm_modal on {w}x{h} terminal must not panic"
        );
    }

    // Empty Step::Programs & WizardStep::Programs key fuzzing must not panic
    let mut empty_state = AppState::new(AppConfig::default());
    empty_state.programs.clear();
    empty_state.current_step = Step::Programs;
    for code in [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Char(' '),
        KeyCode::Char('u'),
        KeyCode::Char('d'),
        KeyCode::Delete,
        KeyCode::Enter,
    ] {
        empty_state.handle_key(KeyEvent::from(code));
    }

    // Check wizard + other views on tiny 10x5 and 0x0 areas
    let wiz_state = AppState::new_wizard(AppConfig::default());
    for (w, h) in [(0, 0), (1, 1), (10, 5), (20, 6)] {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| views::render_wizard(f, f.area(), &wiz_state))
            .unwrap();
        t.draw(|f| views::render_palette(f, f.area(), &state))
            .unwrap();
        t.draw(|f| views::render_hooks(f, f.area(), &state))
            .unwrap();
        t.draw(|f| views::render_apply(f, f.area(), &state))
            .unwrap();
        t.draw(|f| views::render_header(f, f.area(), &state))
            .unwrap();
    }
}

#[test]
fn stress_tui_wizard_finish_partial_write_failure_and_deleted_rule_visibility() {
    let tmp = TempDirGuard::new("tui_partial");
    let paths = tmp.registry_paths();

    let r1 = sample_rule("Foot", 1, "foot.tmpl");
    let t1 = sample_tmpl(1);
    let r2 = sample_rule("Kitty", 1, "kitty.tmpl");
    let t2 = sample_tmpl(1);
    let manifest = format!(
        "schema_version = 1\n\n{}{}",
        manifest_entry(
            "foot",
            "Foot",
            "rules/foot.toml",
            "templates/foot.tmpl",
            1,
            1,
            "1.6.0",
            &sha256_hex(r1.as_bytes()),
            &sha256_hex(t1.as_bytes()),
        ),
        manifest_entry(
            "kitty",
            "Kitty",
            "rules/kitty.toml",
            "templates/kitty.tmpl",
            1,
            1,
            "1.6.0",
            &sha256_hex(r2.as_bytes()),
            &sha256_hex(t2.as_bytes()),
        )
    );
    let mut map = HashMap::new();
    map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(manifest));
    map.insert(format!("{OFFICIAL_REPO_URL}/rules/foot.toml"), Ok(r1));
    map.insert(format!("{OFFICIAL_REPO_URL}/templates/foot.tmpl"), Ok(t1));
    map.insert(format!("{OFFICIAL_REPO_URL}/rules/kitty.toml"), Ok(r2));
    map.insert(format!("{OFFICIAL_REPO_URL}/templates/kitty.tmpl"), Ok(t2));

    let svc = Arc::new(RegistryService::with_paths_and_transport(
        paths.clone(),
        MockTransport::new(map),
        "1.6.0".into(),
    ));
    svc.ensure_default_repo_config().unwrap();
    svc.sync_indexes().unwrap();

    // 1. Partial failure during WizardStep::Finish when config path is a directory!
    let cfg_dir_blocker = tmp.path.join("config_is_dir.toml");
    fs::create_dir_all(&cfg_dir_blocker).unwrap();

    let mut state = AppState::new_wizard(AppConfig {
        language: termonet::config::Language::En,
        ..Default::default()
    });
    state.custom_config_path = Some(cfg_dir_blocker);
    state.set_registry(svc.clone());
    state.enter_wizard_programs_step();

    // Check "foot"
    if let Some(ref mut w) = state.wizard_state {
        w.step = WizardStep::Programs;
        w.selected_idx = 0;
    }
    state.toggle_wizard_option();

    // Finish opens confirmation modal before committing; confirm writes files, then config save fails safely.
    state.finish_wizard().unwrap();
    assert!(state.confirm_modal.is_some());
    assert!(state.is_wizard);
    assert!(!paths.rules_dir.join("foot.toml").exists());

    state.handle_key(KeyEvent::from(KeyCode::Char('y')));
    assert!(state.confirm_modal.is_none());
    assert!(state.is_wizard);
    assert!(state.status_is_error);
    assert!(paths.installed_file.exists());
    assert!(paths.rules_dir.join("foot.toml").exists());

    // 2. Delete rules/foot.toml while foot is in installed.toml -> Step::Programs must still show it
    fs::remove_file(paths.rules_dir.join("foot.toml")).unwrap();
    state.refresh_catalog();
    assert!(state.programs.iter().any(|p| p.id == "foot"));
    assert!(
        state
            .wizard_state
            .as_ref()
            .is_some_and(|w| w.catalog.iter().any(|c| c.id == "foot"))
    );
}

#[test]
fn stress_tui_force_update_modal_with_errors_status_flag() {
    let tmp = TempDirGuard::new("force_modal_err");
    let paths = tmp.registry_paths();

    let r1 = sample_rule("Foot", 1, "foot.tmpl");
    let t1 = sample_tmpl(1);
    let m1 = format!(
        "schema_version = 1\n\n{}",
        manifest_entry(
            "foot",
            "Foot",
            "rules/foot.toml",
            "templates/foot.tmpl",
            1,
            1,
            "1.6.0",
            &sha256_hex(r1.as_bytes()),
            &sha256_hex(t1.as_bytes()),
        )
    );
    let mut map = HashMap::new();
    map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(m1));
    map.insert(
        format!("{OFFICIAL_REPO_URL}/rules/foot.toml"),
        Ok(r1.clone()),
    );
    map.insert(
        format!("{OFFICIAL_REPO_URL}/templates/foot.tmpl"),
        Ok(t1.clone()),
    );

    let mock = MockTransport::new(map);
    let svc = Arc::new(RegistryService::with_paths_and_transport(
        paths.clone(),
        mock.clone(),
        "1.6.0".into(),
    ));
    svc.ensure_default_repo_config().unwrap();
    svc.sync_indexes().unwrap();
    svc.install_programs(&["foot".into()], false, false)
        .unwrap();

    // Bump upstream to v2, user modifies locally, AND upstream rule download fails!
    let r2 = sample_rule("Foot", 2, "foot.tmpl");
    let m2 = format!(
        "schema_version = 1\n\n{}",
        manifest_entry(
            "foot",
            "Foot",
            "rules/foot.toml",
            "templates/foot.tmpl",
            2,
            1,
            "1.6.0",
            &sha256_hex(r2.as_bytes()),
            &sha256_hex(t1.as_bytes()),
        )
    );
    {
        let mut g = mock.responses.lock().unwrap();
        g.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(m2));
        g.insert(
            format!("{OFFICIAL_REPO_URL}/rules/foot.toml"),
            Err("500 internal server error".into()),
        );
    }
    fs::write(paths.templates_dir.join("foot.tmpl"), "modified\n").unwrap();

    let mut state = AppState::new(AppConfig {
        language: termonet::config::Language::En,
        ..Default::default()
    });
    state.set_registry(svc);
    state.refresh_catalog();
    state.current_step = Step::Programs;

    // Trigger 'u' -> opens ForceUpdatePrograms modal
    state.handle_key(KeyEvent::from(KeyCode::Char('u')));
    assert!(state.confirm_modal.is_some());

    // Confirm with 'y' -> update fails in report.errors, so status_is_error must be true
    state.handle_key(KeyEvent::from(KeyCode::Char('y')));
    assert!(state.confirm_modal.is_none());
    assert!(
        state.status_is_error,
        "ForceUpdatePrograms failure must set status_is_error=true, status={:?}",
        state.status_message
    );
}

#[test]
fn stress_config_v3_migration_and_auto_update_edge_cases() {
    let tmp = TempDirGuard::new("cfg_v3");
    let cfg_path = tmp.path.join("config.toml");

    // v1, v2, v3, custom minimal, and broken migrations
    let v2 = "# Termonet Configuration\n# Schema Version: 2\nlanguage = \"en\"\ntui_theme = \"light\"\nfont_mode = \"standard\"\ndisplay_server = \"x11\"\nwallpaper_backend = \"feh\"\nenabled_programs = [\"kitty\"]\ncreate_backups = true\n";
    assert!(AppConfig::needs_migration(v2));
    let parsed_v2: AppConfig = toml::from_str(v2).unwrap();
    assert!(parsed_v2.auto_update_rules);
    assert!(parsed_v2.create_backups);
    parsed_v2.save_to_path(&cfg_path).unwrap();
    let migrated_text = fs::read_to_string(&cfg_path).unwrap();
    assert!(!AppConfig::needs_migration(&migrated_text));

    // Auto update skips when disabled, interval not elapsed, or nothing installed
    let paths = tmp.registry_paths();
    let svc = RegistryService::with_paths_and_transport(
        paths,
        MockTransport::new(HashMap::new()),
        "1.6.0".into(),
    );
    let mut cfg = AppConfig::default();
    assert_eq!(
        svc.maybe_auto_update(&cfg, 1_000_000),
        AutoUpdateOutcome::SkippedNothingInstalled
    );
    cfg.auto_update_rules = false;
    assert_eq!(
        svc.maybe_auto_update(&cfg, 1_000_000),
        AutoUpdateOutcome::SkippedDisabled
    );
}
