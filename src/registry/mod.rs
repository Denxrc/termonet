//! External application template registry management.

pub mod client;
pub mod types;

pub use client::{HttpTransport, MAX_ASSET_BYTES, MAX_MANIFEST_BYTES, RegistryTransport};
pub use types::{
    CatalogEntry, CatalogStatus, InstallPreview, InstalledEntry, InstalledManifest,
    RegistryManifest, RegistryProgram, RepoConfig, UpdateReport, is_version_compatible,
    sanitize_relative_repo_path, sha256_hex, validate_program_id,
};

use crate::config::{AppConfig, AppPaths, AppStateCache};
use crate::error::{Result, TermonetError};
use crate::template::{ProgramRule, TemplateEngine, create_safety_backup, safe_write};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default official repository identifier.
pub const OFFICIAL_REPO_NAME: &str = "official";
/// Default raw GitHub base URL for the official `termonet-apps` repository.
pub const OFFICIAL_REPO_URL: &str = "https://raw.githubusercontent.com/Denxrc/termonet-apps/main";
/// Weekly auto-update interval in seconds (7 days).
pub const WEEKLY_SYNC_INTERVAL_SECS: u64 = 7 * 24 * 60 * 60;

/// Known unmodified SHA-256 checksums for v1.5.2 built-in assets (used for seamless v1.5 -> v1.6 adoption).
const LEGACY_V1_5_ASSETS: &[(&str, u32, &str, u32, &str)] = &[
    (
        "hyprland",
        2,
        "862aece3cef5f270ebf88f8bc2ea7fdf956ef0c9f26734787ca024f1f76e27c7",
        4,
        "a9c3eb610a041e450f5b2145d3f401ee77a6ea5ebba3edb59587d3d61efdf8f6",
    ),
    (
        "kitty",
        2,
        "01011b49f2f6c29db35a8fa80424b05ca532885d254b6c267bd1e501680d63db",
        4,
        "c618bad0d03722dd58653d23f2f52f0a90f60d2d20bf056023e282338e26ad4a",
    ),
    (
        "rofi",
        2,
        "9a1c2fc8a19e81d32374e3bdd29e5eaecbb28eacff856e17f0f1ec8a43b1441f",
        4,
        "c68e586b3fb129c5dea9d5b0beb23f5a940c661b8e45392ce966d5e684b91238",
    ),
    (
        "waybar",
        2,
        "8c3a648def7246e818905d0d98156b8aa18df5e2e9db818e2a9e50ca7630952e",
        4,
        "8f8f48566e9d6f60341c9490e5a7842fc0c2f53565ac9eeea0a50479be89abcb",
    ),
];

/// Filesystem paths operated on by `RegistryService`.
#[derive(Debug, Clone)]
pub struct RegistryPaths {
    pub repos_dir: PathBuf,
    pub rules_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub installed_file: PathBuf,
    pub state_file: PathBuf,
}

impl RegistryPaths {
    /// Resolves canonical XDG paths for the active user.
    pub fn from_system() -> Result<Self> {
        Ok(Self {
            repos_dir: AppPaths::repos_dir()?,
            rules_dir: AppPaths::rules_dir()?,
            templates_dir: AppPaths::templates_dir()?,
            cache_dir: AppPaths::registry_cache_dir()?,
            installed_file: AppPaths::installed_file()?,
            state_file: AppPaths::state_file()?,
        })
    }
}

/// Validated in-memory program package ready to be previewed and committed to disk.
#[derive(Debug, Clone)]
pub struct PreparedInstall {
    pub preview: InstallPreview,
    pub rule_bytes: Vec<u8>,
    pub template_bytes: Vec<u8>,
    pub parsed_rule: ProgramRule,
}

/// Outcome of the weekly automatic registry update check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoUpdateOutcome {
    SkippedDisabled,
    SkippedIntervalNotElapsed,
    SkippedNothingInstalled,
    Completed(UpdateReport),
    FailedNonFatal(String),
}

/// High-level service managing template repositories, local tracking, and safe installation.
pub struct RegistryService<T: RegistryTransport> {
    paths: RegistryPaths,
    transport: T,
    current_version: String,
}

impl RegistryService<HttpTransport> {
    /// Constructs a `RegistryService` wired to standard XDG paths and HTTPS transport.
    pub fn from_system_paths() -> Result<Self> {
        Ok(Self::with_paths_and_transport(
            RegistryPaths::from_system()?,
            HttpTransport::new(),
            env!("CARGO_PKG_VERSION").to_string(),
        ))
    }
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn join_repo_url(base_url: &str, rel_path: &str) -> String {
    format!("{}/{}", base_url.trim_end_matches('/'), rel_path)
}

fn normalize_rule_template_field(rule_text: &str, expected_template: &str) -> Result<Vec<u8>> {
    let mut table: toml::Table = toml::from_str(rule_text)?;
    let needs_update = table
        .get("template")
        .and_then(toml::Value::as_str)
        .is_none_or(|t| t != expected_template);

    if !needs_update {
        return Ok(rule_text.as_bytes().to_vec());
    }

    table.insert(
        "template".to_string(),
        toml::Value::String(expected_template.to_string()),
    );
    let serialized = toml::to_string_pretty(&table)?;
    Ok(serialized.into_bytes())
}

fn is_entry_locally_modified(paths: &RegistryPaths, entry: &InstalledEntry) -> bool {
    let rule_path = paths.rules_dir.join(&entry.rule_file);
    let tmpl_path = paths.templates_dir.join(&entry.template_file);

    let Ok(rule_bytes) = fs::read(rule_path) else {
        return true;
    };
    let Ok(tmpl_bytes) = fs::read(tmpl_path) else {
        return true;
    };

    sha256_hex(&rule_bytes) != entry.rule_sha256 || sha256_hex(&tmpl_bytes) != entry.template_sha256
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).map_err(|err| {
        TermonetError::Registry(format!(
            "Refusing non-UTF-8 content for {}: {}",
            path.display(),
            err
        ))
    })?;
    safe_write(path, text, false)
}

impl<T: RegistryTransport> RegistryService<T> {
    /// Constructs a `RegistryService` with explicit paths, transport, and binary semver.
    #[must_use]
    pub fn with_paths_and_transport(
        paths: RegistryPaths,
        transport: T,
        current_version: String,
    ) -> Self {
        Self {
            paths,
            transport,
            current_version,
        }
    }

    /// Returns a reference to the configured registry paths.
    #[must_use]
    pub fn paths(&self) -> &RegistryPaths {
        &self.paths
    }

    /// Writes `official.toml` in `repos_dir` once if it does not already exist.
    pub fn ensure_default_repo_config(&self) -> Result<()> {
        let official_path = self.paths.repos_dir.join("official.toml");
        if official_path.exists() {
            return Ok(());
        }

        if let Err(err) = fs::create_dir_all(&self.paths.repos_dir) {
            if err.kind() == std::io::ErrorKind::PermissionDenied || err.raw_os_error() == Some(30)
            {
                return Ok(());
            }
            return Err(err.into());
        }

        let default_toml = format!(
            "# Termonet Official Apps Registry\nname = \"{OFFICIAL_REPO_NAME}\"\nurl = \"{OFFICIAL_REPO_URL}\"\npriority = 0\nenabled = true\n"
        );
        if let Err(err) = safe_write(&official_path, &default_toml, false) {
            if let TermonetError::Io(ref io_err) = err
                && (io_err.kind() == std::io::ErrorKind::PermissionDenied
                    || io_err.raw_os_error() == Some(30))
            {
                return Ok(());
            }
            return Err(err);
        }
        Ok(())
    }

    /// Loads repository source configs from `repos_dir`, sorting by descending priority.
    pub fn load_repos(&self) -> Result<(Vec<RepoConfig>, Vec<String>)> {
        let mut repos = Vec::new();
        let mut warnings = Vec::new();

        if !self.paths.repos_dir.is_dir() {
            return Ok((repos, warnings));
        }

        let mut files: Vec<PathBuf> = fs::read_dir(&self.paths.repos_dir)?
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
            .collect();
        files.sort();

        let mut seen_names = HashSet::new();
        for file in files {
            let content = match fs::read_to_string(&file) {
                Ok(c) => c,
                Err(err) => {
                    warnings.push(format!("Failed to read repo {}: {}", file.display(), err));
                    continue;
                }
            };
            let mut repo: RepoConfig = match toml::from_str(&content) {
                Ok(r) => r,
                Err(err) => {
                    warnings.push(format!("Failed to parse repo {}: {}", file.display(), err));
                    continue;
                }
            };

            repo.name = repo.name.trim().to_string();
            repo.url = repo.url.trim().trim_end_matches('/').to_string();

            if let Err(err) = validate_program_id(&repo.name) {
                warnings.push(format!("Invalid repo name in {}: {}", file.display(), err));
                continue;
            }
            if repo.url.is_empty() {
                warnings.push(format!("Empty repo url in {}", file.display()));
                continue;
            }
            if !seen_names.insert(repo.name.clone()) {
                warnings.push(format!(
                    "Duplicate repo name '{}' in {}; ignoring duplicate",
                    repo.name,
                    file.display()
                ));
                continue;
            }

            repos.push(repo);
        }

        repos.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.name.cmp(&b.name)));
        Ok((repos, warnings))
    }

    fn load_installed_manifest(&self) -> Result<InstalledManifest> {
        if !self.paths.installed_file.exists() {
            return Ok(InstalledManifest::default());
        }
        let content = fs::read_to_string(&self.paths.installed_file)?;
        let manifest: InstalledManifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    fn save_installed_manifest(&self, manifest: &InstalledManifest) -> Result<()> {
        let mut sorted = manifest.clone();
        sorted.schema_version = 1;
        sorted.installed.sort_by(|a, b| a.id.cmp(&b.id));
        let serialized = toml::to_string_pretty(&sorted)?;
        safe_write(&self.paths.installed_file, &serialized, false)
    }

    /// Adopts unmodified v1.5.2 default rules/templates into `installed.toml` once on upgrade.
    pub fn adopt_legacy_v1_5_assets(&self) -> Result<Vec<String>> {
        if self.paths.installed_file.exists() {
            return Ok(Vec::new());
        }

        let mut adopted = Vec::new();
        let mut entries = Vec::new();

        for &(id, rule_ver, rule_sha, tmpl_ver, tmpl_sha) in LEGACY_V1_5_ASSETS {
            let rule_file = format!("{id}.toml");
            let tmpl_file = format!("{id}.tmpl");
            let rule_path = self.paths.rules_dir.join(&rule_file);
            let tmpl_path = self.paths.templates_dir.join(&tmpl_file);

            if !rule_path.is_file() || !tmpl_path.is_file() {
                continue;
            }

            let Ok(rule_bytes) = fs::read(&rule_path) else {
                continue;
            };
            let Ok(tmpl_bytes) = fs::read(&tmpl_path) else {
                continue;
            };

            if sha256_hex(&rule_bytes) == rule_sha && sha256_hex(&tmpl_bytes) == tmpl_sha {
                adopted.push(id.to_string());
                entries.push(InstalledEntry {
                    id: id.to_string(),
                    repo: OFFICIAL_REPO_NAME.to_string(),
                    rule_file,
                    template_file: tmpl_file,
                    rule_version: rule_ver,
                    template_version: tmpl_ver,
                    rule_sha256: rule_sha.to_string(),
                    template_sha256: tmpl_sha.to_string(),
                });
            }
        }

        if !entries.is_empty() {
            self.save_installed_manifest(&InstalledManifest {
                schema_version: 1,
                installed: entries,
            })?;
        }

        Ok(adopted)
    }

    fn validate_manifest(&self, manifest: &RegistryManifest, repo_name: &str) -> Result<()> {
        if manifest.schema_version != 1 {
            return Err(TermonetError::Registry(format!(
                "Unsupported registry schema_version {} in repo '{}'",
                manifest.schema_version, repo_name
            )));
        }
        let mut seen_ids = HashSet::new();
        for prog in &manifest.programs {
            validate_program_id(&prog.id)?;
            if !seen_ids.insert(prog.id.as_str()) {
                return Err(TermonetError::Registry(format!(
                    "Duplicate program id '{}' in repo '{}'",
                    prog.id, repo_name
                )));
            }
            sanitize_relative_repo_path(&prog.rule)?;
            sanitize_relative_repo_path(&prog.template)?;
        }
        Ok(())
    }

    fn cache_path_for_repo(&self, repo_name: &str) -> PathBuf {
        self.paths.cache_dir.join(format!("{repo_name}.toml"))
    }

    /// Fetches `registry.toml` from all enabled repositories and saves them to cache.
    pub fn sync_indexes(&self) -> Result<Vec<String>> {
        let (repos, mut warnings) = self.load_repos()?;
        let enabled_repos: Vec<RepoConfig> = repos.into_iter().filter(|r| r.enabled).collect();
        if enabled_repos.is_empty() {
            return Err(TermonetError::Registry(
                "No enabled template repositories configured".into(),
            ));
        }

        let mut synced_count = 0usize;
        let mut errors = Vec::new();

        for repo in &enabled_repos {
            let url = join_repo_url(&repo.url, "registry.toml");
            match self.transport.fetch_text(&url, MAX_MANIFEST_BYTES) {
                Ok(raw) => match toml::from_str::<RegistryManifest>(&raw) {
                    Ok(manifest) => match self.validate_manifest(&manifest, &repo.name) {
                        Ok(()) => {
                            let cache_path = self.cache_path_for_repo(&repo.name);
                            safe_write(&cache_path, &raw, false)?;
                            synced_count += 1;
                        }
                        Err(err) => errors.push(err.to_string()),
                    },
                    Err(err) => {
                        errors.push(format!("Invalid registry.toml from '{}': {}", repo.name, err))
                    }
                },
                Err(err) => errors.push(format!("Failed to sync repo '{}': {}", repo.name, err)),
            }
        }

        if synced_count == 0 {
            return Err(TermonetError::Registry(errors.join("; ")));
        }

        warnings.extend(errors);

        let mut state = AppStateCache::load_from_path(&self.paths.state_file).unwrap_or_default();
        state.last_registry_sync_at = Some(current_epoch_secs());
        let _ = state.save_to_path(&self.paths.state_file);

        Ok(warnings)
    }

    /// Syncs indexes only if no cached manifest exists yet for enabled repositories.
    pub fn sync_indexes_if_empty(&self) -> Result<Vec<String>> {
        let (repos, _) = self.load_repos()?;
        let has_any_cache = repos
            .iter()
            .filter(|r| r.enabled)
            .any(|r| self.cache_path_for_repo(&r.name).is_file());
        if has_any_cache {
            Ok(Vec::new())
        } else {
            self.sync_indexes()
        }
    }

    fn load_cached_manifests(
        &self,
        repos: &[RepoConfig],
    ) -> (HashMap<String, RegistryManifest>, Vec<String>) {
        let mut map = HashMap::new();
        let mut warnings = Vec::new();
        for repo in repos.iter().filter(|r| r.enabled) {
            let cache_path = self.cache_path_for_repo(&repo.name);
            if !cache_path.is_file() {
                continue;
            }
            match fs::read_to_string(&cache_path)
                .map_err(TermonetError::from)
                .and_then(|s| toml::from_str::<RegistryManifest>(&s).map_err(TermonetError::from))
                .and_then(|m| {
                    self.validate_manifest(&m, &repo.name)?;
                    Ok(m)
                }) {
                Ok(manifest) => {
                    map.insert(repo.name.clone(), manifest);
                }
                Err(err) => {
                    warnings.push(format!(
                        "Ignored corrupted cache for '{}': {}",
                        repo.name, err
                    ));
                }
            }
        }
        (map, warnings)
    }

    /// Merges installed state, cached remote indexes, and local custom rules into a catalog.
    pub fn catalog(&self) -> Result<(Vec<CatalogEntry>, Vec<String>)> {
        let (repos, mut warnings) = self.load_repos()?;
        let (manifests, cache_warnings) = self.load_cached_manifests(&repos);
        warnings.extend(cache_warnings);

        let installed = self.load_installed_manifest().unwrap_or_default();
        let installed_by_id: HashMap<&str, &InstalledEntry> = installed
            .installed
            .iter()
            .map(|e| (e.id.as_str(), e))
            .collect();

        let (local_rules, rule_warnings) =
            ProgramRule::load_all_with_warnings_from(Some(&self.paths.rules_dir))?;
        warnings.extend(rule_warnings);
        let local_by_id: HashMap<String, ProgramRule> = local_rules
            .into_iter()
            .map(|r| (r.effective_id(), r))
            .collect();

        let mut entries_by_id: HashMap<String, CatalogEntry> = HashMap::new();

        // 1. Add tracked installed entries pinned to their origin repo
        for entry in &installed.installed {
            let upstream = manifests
                .get(&entry.repo)
                .and_then(|m| m.programs.iter().find(|p| p.id == entry.id));

            let locally_modified = is_entry_locally_modified(&self.paths, entry);
            let update_available = upstream.and_then(|u| {
                let compat = is_version_compatible(&u.min_termonet_version, &self.current_version)
                    .unwrap_or(false);
                if compat
                    && (u.rule_version > entry.rule_version
                        || u.template_version > entry.template_version)
                {
                    Some((u.rule_version, u.template_version))
                } else {
                    None
                }
            });

            let local_rule = local_by_id.get(&entry.id);
            let name = local_rule
                .map(|r| r.name.clone())
                .or_else(|| upstream.map(|u| u.name.clone()))
                .unwrap_or_else(|| entry.id.clone());

            entries_by_id.insert(
                entry.id.clone(),
                CatalogEntry {
                    id: entry.id.clone(),
                    name,
                    category: upstream.map(|u| u.category.clone()).unwrap_or_default(),
                    description: upstream.map(|u| u.description.clone()).unwrap_or_default(),
                    description_ru: upstream
                        .map(|u| u.description_ru.clone())
                        .unwrap_or_default(),
                    status: CatalogStatus::Installed {
                        repo: entry.repo.clone(),
                        update_available,
                        locally_modified,
                    },
                    output_path: local_rule
                        .map(|r| r.output_path.to_string_lossy().to_string()),
                    target_config: local_rule
                        .and_then(|r| r.target_config.as_ref())
                        .map(|p| p.to_string_lossy().to_string()),
                    reload_cmd: local_rule.and_then(|r| r.reload_cmd.clone()),
                    min_termonet_version: upstream.map(|u| u.min_termonet_version.clone()),
                },
            );
        }

        // 2. Add untracked local custom rules
        for (id, rule) in &local_by_id {
            if installed_by_id.contains_key(id.as_str()) {
                continue;
            }
            entries_by_id.insert(
                id.clone(),
                CatalogEntry {
                    id: id.clone(),
                    name: rule.name.clone(),
                    category: "local".to_string(),
                    description: "Custom local rule".to_string(),
                    description_ru: "Пользовательское локальное правило".to_string(),
                    status: CatalogStatus::LocalCustom,
                    output_path: Some(rule.output_path.to_string_lossy().to_string()),
                    target_config: rule
                        .target_config
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string()),
                    reload_cmd: rule.reload_cmd.clone(),
                    min_termonet_version: None,
                },
            );
        }

        // 3. Add available uninstalled programs deduplicated by repo priority order
        for repo in repos.iter().filter(|r| r.enabled) {
            let Some(manifest) = manifests.get(&repo.name) else {
                continue;
            };
            for prog in &manifest.programs {
                if entries_by_id.contains_key(&prog.id) {
                    continue;
                }
                let compatible =
                    is_version_compatible(&prog.min_termonet_version, &self.current_version)
                        .unwrap_or(false);
                entries_by_id.insert(
                    prog.id.clone(),
                    CatalogEntry {
                        id: prog.id.clone(),
                        name: prog.name.clone(),
                        category: prog.category.clone(),
                        description: prog.description.clone(),
                        description_ru: prog.description_ru.clone(),
                        status: CatalogStatus::Available {
                            repo: repo.name.clone(),
                            compatible,
                        },
                        output_path: None,
                        target_config: None,
                        reload_cmd: None,
                        min_termonet_version: Some(prog.min_termonet_version.clone()),
                    },
                );
            }
        }

        let mut list: Vec<CatalogEntry> = entries_by_id.into_values().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        Ok((list, warnings))
    }

    fn resolve_target<'a>(
        &self,
        spec: &str,
        repos: &'a [RepoConfig],
        manifests: &'a HashMap<String, RegistryManifest>,
        installed: &InstalledManifest,
        force: bool,
    ) -> Result<(&'a RepoConfig, &'a RegistryProgram)> {
        let (explicit_repo, raw_id) = match spec.split_once('/') {
            Some((r, id)) => {
                validate_program_id(r)?;
                validate_program_id(id)?;
                (Some(r), id)
            }
            None => {
                validate_program_id(spec)?;
                (None, spec)
            }
        };

        if let Some(repo_name) = explicit_repo {
            let repo = repos
                .iter()
                .find(|r| r.enabled && r.name == repo_name)
                .ok_or_else(|| {
                    TermonetError::Registry(format!(
                        "Repository '{repo_name}' is not configured or is disabled"
                    ))
                })?;
            let prog = manifests
                .get(&repo.name)
                .and_then(|m| m.programs.iter().find(|p| p.id == raw_id))
                .ok_or_else(|| {
                    TermonetError::Registry(format!(
                        "Program '{raw_id}' not found in repository '{repo_name}'"
                    ))
                })?;
            return Ok((repo, prog));
        }

        // If already installed and !force, pin to its installed repo
        if !force
            && let Some(entry) = installed.installed.iter().find(|e| e.id == raw_id)
            && let Some(repo) = repos.iter().find(|r| r.enabled && r.name == entry.repo)
            && let Some(prog) = manifests
                .get(&repo.name)
                .and_then(|m| m.programs.iter().find(|p| p.id == raw_id))
        {
            return Ok((repo, prog));
        }

        for repo in repos.iter().filter(|r| r.enabled) {
            if let Some(prog) = manifests
                .get(&repo.name)
                .and_then(|m| m.programs.iter().find(|p| p.id == raw_id))
            {
                return Ok((repo, prog));
            }
        }

        Err(TermonetError::Registry(format!(
            "Program '{raw_id}' not found in any enabled repository"
        )))
    }

    /// Fetches and validates packages in memory so the UI/CLI can preview warnings before writing.
    pub fn prepare_install(&self, specs: &[String], force: bool) -> Result<Vec<PreparedInstall>> {
        if specs.is_empty() {
            return Ok(Vec::new());
        }

        let _ = self.sync_indexes_if_empty();
        let (repos, _) = self.load_repos()?;
        let (manifests, _) = self.load_cached_manifests(&repos);
        let installed = self.load_installed_manifest().unwrap_or_default();
        let engine = TemplateEngine::new();

        let mut prepared = Vec::with_capacity(specs.len());
        let mut seen_ids = HashSet::new();

        for spec in specs {
            let (repo, prog) = self.resolve_target(spec, &repos, &manifests, &installed, force)?;
            if !seen_ids.insert(prog.id.clone()) {
                continue;
            }

            if !is_version_compatible(&prog.min_termonet_version, &self.current_version)? {
                return Err(TermonetError::Registry(format!(
                    "Program '{}' requires Termonet >= {} (current version is {})",
                    prog.id, prog.min_termonet_version, self.current_version
                )));
            }

            let rule_file_name = format!("{}.toml", prog.id);
            let tmpl_file_name = format!("{}.tmpl", prog.id);
            let local_rule_path = self.paths.rules_dir.join(&rule_file_name);
            let tracked = installed.installed.iter().find(|e| e.id == prog.id);

            let has_local_conflict = match tracked {
                Some(entry) => {
                    entry.repo != repo.name || is_entry_locally_modified(&self.paths, entry)
                }
                None => local_rule_path.exists(),
            };

            if has_local_conflict && !force {
                return Err(TermonetError::Registry(format!(
                    "Local rule or edit conflict for '{}'; pass --force to overwrite",
                    prog.id
                )));
            }

            let rule_rel = sanitize_relative_repo_path(&prog.rule)?;
            let tmpl_rel = sanitize_relative_repo_path(&prog.template)?;
            let rule_url = join_repo_url(&repo.url, rule_rel);
            let tmpl_url = join_repo_url(&repo.url, tmpl_rel);

            let raw_rule = self.transport.fetch_text(&rule_url, MAX_ASSET_BYTES)?;
            let raw_tmpl = self.transport.fetch_text(&tmpl_url, MAX_ASSET_BYTES)?;

            let actual_rule_sha = sha256_hex(raw_rule.as_bytes());
            if actual_rule_sha != prog.rule_sha256 {
                return Err(TermonetError::Registry(format!(
                    "SHA-256 mismatch for '{}': expected {}, got {}",
                    rule_url, prog.rule_sha256, actual_rule_sha
                )));
            }

            let actual_tmpl_sha = sha256_hex(raw_tmpl.as_bytes());
            if actual_tmpl_sha != prog.template_sha256 {
                return Err(TermonetError::Registry(format!(
                    "SHA-256 mismatch for '{}': expected {}, got {}",
                    tmpl_url, prog.template_sha256, actual_tmpl_sha
                )));
            }

            let mut parsed_rule = ProgramRule::from_toml(&raw_rule)?;
            parsed_rule.id = prog.id.clone();
            engine.validate_syntax(&raw_tmpl)?;

            let normalized_rule_bytes = normalize_rule_template_field(&raw_rule, &tmpl_file_name)?;
            parsed_rule.template = tmpl_file_name;

            prepared.push(PreparedInstall {
                preview: InstallPreview {
                    id: prog.id.clone(),
                    repo: repo.name.clone(),
                    name: parsed_rule.name.clone(),
                    output_path: parsed_rule.output_path.to_string_lossy().to_string(),
                    target_config: parsed_rule
                        .target_config
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string()),
                    reload_cmd: parsed_rule.reload_cmd.clone(),
                    has_local_conflict,
                    rule_version: prog.rule_version,
                    template_version: prog.template_version,
                },
                rule_bytes: normalized_rule_bytes,
                template_bytes: raw_tmpl.into_bytes(),
                parsed_rule,
            });
        }

        Ok(prepared)
    }

    /// Atomically commits pre-validated packages to disk with rollback if a pair write fails.
    pub fn commit_install(
        &self,
        prepared: &[PreparedInstall],
        force: bool,
        create_backups: bool,
    ) -> Result<Vec<String>> {
        if prepared.is_empty() {
            return Ok(Vec::new());
        }

        let mut manifest = self.load_installed_manifest().unwrap_or_default();
        let mut installed_ids = Vec::with_capacity(prepared.len());

        for item in prepared {
            let id = &item.preview.id;
            let rule_file = format!("{id}.toml");
            let tmpl_file = format!("{id}.tmpl");
            let rule_path = self.paths.rules_dir.join(&rule_file);
            let tmpl_path = self.paths.templates_dir.join(&tmpl_file);

            let prev_rule_bytes = fs::read(&rule_path).ok();

            if force && create_backups {
                if rule_path.exists() {
                    let _ = create_safety_backup(&rule_path);
                }
                if tmpl_path.exists() {
                    let _ = create_safety_backup(&tmpl_path);
                }
            }

            write_bytes_atomic(&rule_path, &item.rule_bytes)?;

            if let Err(err) = write_bytes_atomic(&tmpl_path, &item.template_bytes) {
                // Roll back the rule file so no half-installed pair remains on disk
                match prev_rule_bytes {
                    Some(old) => {
                        let _ = write_bytes_atomic(&rule_path, &old);
                    }
                    None => {
                        let _ = fs::remove_file(&rule_path);
                    }
                }
                return Err(err);
            }

            let new_entry = InstalledEntry {
                id: id.clone(),
                repo: item.preview.repo.clone(),
                rule_file,
                template_file: tmpl_file,
                rule_version: item.preview.rule_version,
                template_version: item.preview.template_version,
                rule_sha256: sha256_hex(&item.rule_bytes),
                template_sha256: sha256_hex(&item.template_bytes),
            };

            if let Some(existing) = manifest.installed.iter_mut().find(|e| e.id == *id) {
                *existing = new_entry;
            } else {
                manifest.installed.push(new_entry);
            }

            self.save_installed_manifest(&manifest)?;
            installed_ids.push(id.clone());
        }

        Ok(installed_ids)
    }

    /// Convenience helper calling `prepare_install` followed by `commit_install`.
    pub fn install_programs(
        &self,
        specs: &[String],
        force: bool,
        create_backups: bool,
    ) -> Result<Vec<String>> {
        let prepared = self.prepare_install(specs, force)?;
        self.commit_install(&prepared, force, create_backups)
    }

    /// Updates all tracked installed programs from their pinned repositories.
    pub fn update_installed(&self, force: bool, create_backups: bool) -> Result<UpdateReport> {
        let sync_warnings = self.sync_indexes()?;
        let mut report = UpdateReport {
            errors: sync_warnings,
            ..Default::default()
        };

        let installed = self.load_installed_manifest().unwrap_or_default();
        if installed.installed.is_empty() {
            return Ok(report);
        }

        let (repos, _) = self.load_repos()?;
        let (manifests, _) = self.load_cached_manifests(&repos);

        for entry in &installed.installed {
            let Some(repo) = repos.iter().find(|r| r.enabled && r.name == entry.repo) else {
                report.errors.push(format!(
                    "Pinned repository '{}' for '{}' is disabled or missing",
                    entry.repo, entry.id
                ));
                continue;
            };
            let Some(upstream) = manifests
                .get(&repo.name)
                .and_then(|m| m.programs.iter().find(|p| p.id == entry.id))
            else {
                report.errors.push(format!(
                    "Program '{}' not found in pinned repository '{}'",
                    entry.id, entry.repo
                ));
                continue;
            };

            match is_version_compatible(&upstream.min_termonet_version, &self.current_version) {
                Ok(true) => {}
                Ok(false) => {
                    report.skipped_incompatible.push(entry.id.clone());
                    continue;
                }
                Err(err) => {
                    report.errors.push(err.to_string());
                    continue;
                }
            }

            let needs_upgrade = upstream.rule_version > entry.rule_version
                || upstream.template_version > entry.template_version;

            if !needs_upgrade && !force {
                report.up_to_date.push(entry.id.clone());
                continue;
            }

            if !force && is_entry_locally_modified(&self.paths, entry) {
                report.skipped_modified.push(entry.id.clone());
                continue;
            }

            let spec = format!("{}/{}", entry.repo, entry.id);
            match self.install_programs(&[spec], true, create_backups) {
                Ok(_) => report.updated.push(entry.id.clone()),
                Err(err) => report
                    .errors
                    .push(format!("Failed updating '{}': {}", entry.id, err)),
            }
        }

        Ok(report)
    }

    /// Removes registry-tracked rule/template files and unregisters the entry from `installed.toml`.
    pub fn remove_program(&self, id: &str) -> Result<()> {
        validate_program_id(id)?;
        let mut manifest = self.load_installed_manifest()?;
        let pos = manifest
            .installed
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| {
                TermonetError::Registry(format!(
                    "Program '{id}' is not managed by the registry (custom [local] rules are protected)"
                ))
            })?;

        let entry = manifest.installed.remove(pos);
        let rule_path = self.paths.rules_dir.join(&entry.rule_file);
        let tmpl_path = self.paths.templates_dir.join(&entry.template_file);

        if rule_path.exists() {
            fs::remove_file(&rule_path)?;
        }
        if tmpl_path.exists() {
            fs::remove_file(&tmpl_path)?;
        }

        self.save_installed_manifest(&manifest)?;
        Ok(())
    }

    /// Runs weekly auto-update if enabled, 7 days have elapsed, and tracked apps exist.
    pub fn maybe_auto_update(&self, config: &AppConfig, now_epoch_secs: u64) -> AutoUpdateOutcome {
        if !config.auto_update_rules {
            return AutoUpdateOutcome::SkippedDisabled;
        }

        let installed = match self.load_installed_manifest() {
            Ok(m) if !m.installed.is_empty() => m,
            Ok(_) => return AutoUpdateOutcome::SkippedNothingInstalled,
            Err(err) => return AutoUpdateOutcome::FailedNonFatal(err.to_string()),
        };
        let _ = installed;

        let state = AppStateCache::load_from_path(&self.paths.state_file).unwrap_or_default();
        if let Some(last_sync) = state.last_registry_sync_at
            && now_epoch_secs.saturating_sub(last_sync) < WEEKLY_SYNC_INTERVAL_SECS
        {
            return AutoUpdateOutcome::SkippedIntervalNotElapsed;
        }

        match self.update_installed(false, config.create_backups) {
            Ok(report) => AutoUpdateOutcome::Completed(report),
            Err(err) => AutoUpdateOutcome::FailedNonFatal(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct MockTransport {
        responses: HashMap<String, std::result::Result<String, String>>,
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl MockTransport {
        fn new(responses: HashMap<String, std::result::Result<String, String>>) -> Self {
            Self {
                responses,
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl RegistryTransport for MockTransport {
        fn fetch_text(&self, url: &str, max_bytes: usize) -> Result<String> {
            self.calls.lock().unwrap().push(url.to_string());
            match self.responses.get(url) {
                Some(Ok(body)) if body.len() <= max_bytes => Ok(body.clone()),
                Some(Ok(_)) => Err(TermonetError::Registry("exceeds max_bytes".into())),
                Some(Err(msg)) => Err(TermonetError::Registry(msg.clone())),
                None => Err(TermonetError::Registry(format!("404 mock not found: {url}"))),
            }
        }
    }

    fn temp_registry_paths(suffix: &str) -> RegistryPaths {
        let base = std::env::temp_dir().join(format!(
            "termonet_reg_test_{}_{}",
            suffix,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        RegistryPaths {
            repos_dir: base.join("repos"),
            rules_dir: base.join("rules"),
            templates_dir: base.join("templates"),
            cache_dir: base.join("cache"),
            installed_file: base.join("installed.toml"),
            state_file: base.join("state.toml"),
        }
    }

    fn sample_rule(name: &str, version: u32, template: &str) -> String {
        format!(
            "version = {version}\nname = \"{name}\"\ntemplate = \"{template}\"\noutput_path = \"~/.config/app/colors.conf\"\ntarget_config = \"~/.config/app/app.conf\"\nimport_line = \"include colors.conf\"\nreload_cmd = \"killall -SIGUSR1 app\"\n"
        )
    }

    fn sample_tmpl(version: u32) -> String {
        format!("# termonet-version: {version}\nfg = {{{{ active.primary.hex }}}}\n")
    }

    fn build_manifest(entries: &[(&str, &str, u32, u32, &str, &str, &str)]) -> String {
        let mut out = String::from("schema_version = 1\n\n");
        for &(id, name, rv, tv, min_ver, r_sha, t_sha) in entries {
            out.push_str(&format!(
                "[[programs]]\nid = \"{id}\"\nname = \"{name}\"\ncategory = \"terminal\"\ndescription = \"{name} theme\"\ndescription_ru = \"Тема {name}\"\nrule = \"rules/{id}.toml\"\ntemplate = \"templates/{id}.tmpl\"\nrule_version = {rv}\ntemplate_version = {tv}\nmin_termonet_version = \"{min_ver}\"\nrule_sha256 = \"{r_sha}\"\ntemplate_sha256 = \"{t_sha}\"\n\n"
            ));
        }
        out
    }

    #[test]
    fn test_adopt_legacy_v1_5_unmodified_vs_modified_files() {
        let paths = temp_registry_paths("adopt");
        fs::create_dir_all(&paths.rules_dir).unwrap();
        fs::create_dir_all(&paths.templates_dir).unwrap();

        let kitty_rule = concat!(
            "version = 2\n",
            "name = \"Kitty\"\n",
            "template = \"kitty.tmpl\"\n",
            "output_path = \"~/.config/kitty/colors-termonet.conf\"\n",
            "target_config = \"~/.config/kitty/kitty.conf\"\n",
            "import_line = \"include colors-termonet.conf\"\n",
            "import_position = \"bottom\"\n",
            "reload_cmd = \"killall -SIGUSR1 kitty\"\n"
        );
        let kitty_tmpl = concat!(
            "# ==============================================================================\n",
            "# Termonet Kitty Color Scheme\n",
            "# Generated automatically by Termonet\n",
            "# termonet-version: 4\n",
            "# ==============================================================================\n\n",
            "background {{ terminal.background.hex }}\n",
            "foreground {{ terminal.foreground.hex }}\n",
            "selection_background {{ active.secondary_container.hex }}\n",
            "selection_foreground {{ active.on_secondary_container.hex }}\n",
            "cursor {{ terminal.cursor.hex }}\n",
            "cursor_text_color {{ terminal.background.hex }}\n\n",
            "# Tab Bar (Material You)\n",
            "active_tab_foreground {{ active.on_primary_container.hex }}\n",
            "active_tab_background {{ active.primary_container.hex }}\n",
            "inactive_tab_foreground {{ active.on_surface_variant.hex }}\n",
            "inactive_tab_background {{ active.surface_variant.hex }}\n",
            "tab_bar_background {{ active.surface.hex }}\n\n",
            "# Window Borders & Alerts\n",
            "active_border_color {{ active.primary.hex }}\n",
            "inactive_border_color {{ active.surface_variant.hex }}\n",
            "bell_border_color {{ active.error.hex }}\n\n",
            "# URL & Highlights\n",
            "url_color {{ active.primary.hex }}\n\n",
            "# ANSI 16 Colors\n",
            "# Black\n",
            "color0 {{ terminal.color0.hex }}\n",
            "color8 {{ terminal.color8.hex }}\n\n",
            "# Red\n",
            "color1 {{ terminal.color1.hex }}\n",
            "color9 {{ terminal.color9.hex }}\n\n",
            "# Green\n",
            "color2 {{ terminal.color2.hex }}\n",
            "color10 {{ terminal.color10.hex }}\n\n",
            "# Yellow\n",
            "color3 {{ terminal.color3.hex }}\n",
            "color11 {{ terminal.color11.hex }}\n\n",
            "# Blue\n",
            "color4 {{ terminal.color4.hex }}\n",
            "color12 {{ terminal.color12.hex }}\n\n",
            "# Magenta\n",
            "color5 {{ terminal.color5.hex }}\n",
            "color13 {{ terminal.color13.hex }}\n\n",
            "# Cyan\n",
            "color6 {{ terminal.color6.hex }}\n",
            "color14 {{ terminal.color14.hex }}\n\n",
            "# White\n",
            "color7 {{ terminal.color7.hex }}\n",
            "color15 {{ terminal.color15.hex }}\n"
        );
        fs::write(paths.rules_dir.join("kitty.toml"), kitty_rule).unwrap();
        fs::write(paths.templates_dir.join("kitty.tmpl"), kitty_tmpl).unwrap();

        // Modified waybar rule must NOT be adopted
        fs::write(
            paths.rules_dir.join("waybar.toml"),
            "version = 2\nname = \"Waybar\"\ntemplate = \"waybar.tmpl\"\noutput_path = \"~/custom/waybar.css\"\n",
        )
        .unwrap();
        fs::write(paths.templates_dir.join("waybar.tmpl"), "/* waybar */\n").unwrap();

        let svc = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(HashMap::new()),
            "1.6.0".into(),
        );
        let adopted = svc.adopt_legacy_v1_5_assets().unwrap();
        assert_eq!(adopted, vec!["kitty".to_string()]);

        let installed: InstalledManifest =
            toml::from_str(&fs::read_to_string(&paths.installed_file).unwrap()).unwrap();
        assert_eq!(installed.installed.len(), 1);
        assert_eq!(installed.installed[0].id, "kitty");
        assert_eq!(installed.installed[0].repo, OFFICIAL_REPO_NAME);

        let _ = fs::remove_dir_all(paths.repos_dir.parent().unwrap());
    }

    #[test]
    fn test_install_programs_validates_in_memory_and_normalizes_template_name() {
        let paths = temp_registry_paths("install_norm");
        let rule_txt = sample_rule("Foot", 1, "shared_weird_name.tmpl");
        let tmpl_txt = sample_tmpl(1);
        let r_sha = sha256_hex(rule_txt.as_bytes());
        let t_sha = sha256_hex(tmpl_txt.as_bytes());
        let manifest = build_manifest(&[("foot", "Foot", 1, 1, "1.6.0", &r_sha, &t_sha)]);

        let mut map = HashMap::new();
        map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(manifest));
        map.insert(format!("{OFFICIAL_REPO_URL}/rules/foot.toml"), Ok(rule_txt));
        map.insert(
            format!("{OFFICIAL_REPO_URL}/templates/foot.tmpl"),
            Ok(tmpl_txt.clone()),
        );

        let svc = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(map),
            "1.6.0".into(),
        );
        svc.ensure_default_repo_config().unwrap();
        svc.sync_indexes().unwrap();

        let prepared = svc.prepare_install(&["foot".into()], false).unwrap();
        assert_eq!(prepared.len(), 1);
        assert_eq!(
            prepared[0].preview.reload_cmd.as_deref(),
            Some("killall -SIGUSR1 app")
        );

        let installed_ids = svc.commit_install(&prepared, false, false).unwrap();
        assert_eq!(installed_ids, vec!["foot".to_string()]);

        let disk_rule = fs::read_to_string(paths.rules_dir.join("foot.toml")).unwrap();
        let parsed = ProgramRule::from_toml(&disk_rule).unwrap();
        assert_eq!(parsed.template, "foot.tmpl");
        assert_eq!(
            fs::read_to_string(paths.templates_dir.join("foot.tmpl")).unwrap(),
            tmpl_txt
        );

        let _ = fs::remove_dir_all(paths.repos_dir.parent().unwrap());
    }

    #[test]
    fn test_install_programs_rolls_back_rule_if_template_write_fails() {
        let paths = temp_registry_paths("rollback");
        let rule_txt = sample_rule("Foot", 1, "foot.tmpl");
        let tmpl_txt = sample_tmpl(1);
        let r_sha = sha256_hex(rule_txt.as_bytes());
        let t_sha = sha256_hex(tmpl_txt.as_bytes());
        let manifest = build_manifest(&[("foot", "Foot", 1, 1, "1.6.0", &r_sha, &t_sha)]);

        let mut map = HashMap::new();
        map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(manifest));
        map.insert(format!("{OFFICIAL_REPO_URL}/rules/foot.toml"), Ok(rule_txt));
        map.insert(
            format!("{OFFICIAL_REPO_URL}/templates/foot.tmpl"),
            Ok(tmpl_txt),
        );

        // Force templates_dir/foot.tmpl to be a directory so writing template file fails
        fs::create_dir_all(paths.templates_dir.join("foot.tmpl")).unwrap();

        let svc = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(map),
            "1.6.0".into(),
        );
        svc.ensure_default_repo_config().unwrap();
        svc.sync_indexes().unwrap();

        let res = svc.install_programs(&["foot".into()], false, false);
        assert!(res.is_err());
        assert!(
            !paths.rules_dir.join("foot.toml").exists(),
            "Rule file must be rolled back when template write fails"
        );
        assert!(!paths.installed_file.exists());

        let _ = fs::remove_dir_all(paths.repos_dir.parent().unwrap());
    }

    #[test]
    fn test_install_rejects_sha256_mismatch_and_incompatible_version_without_disk_writes() {
        let paths = temp_registry_paths("mismatch");
        let rule_txt = sample_rule("Foot", 1, "foot.tmpl");
        let tmpl_txt = sample_tmpl(1);
        let r_sha = sha256_hex(rule_txt.as_bytes());
        let t_sha = sha256_hex(tmpl_txt.as_bytes());
        let manifest = build_manifest(&[
            ("badhash", "BadHash", 1, 1, "1.6.0", "0000", &t_sha),
            ("future", "Future", 1, 1, "2.0.0", &r_sha, &t_sha),
        ]);

        let mut map = HashMap::new();
        map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(manifest));
        map.insert(
            format!("{OFFICIAL_REPO_URL}/rules/badhash.toml"),
            Ok(rule_txt.clone()),
        );
        map.insert(
            format!("{OFFICIAL_REPO_URL}/templates/badhash.tmpl"),
            Ok(tmpl_txt.clone()),
        );
        map.insert(format!("{OFFICIAL_REPO_URL}/rules/future.toml"), Ok(rule_txt));
        map.insert(
            format!("{OFFICIAL_REPO_URL}/templates/future.tmpl"),
            Ok(tmpl_txt),
        );

        let svc = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(map),
            "1.6.0".into(),
        );
        svc.ensure_default_repo_config().unwrap();
        svc.sync_indexes().unwrap();

        assert!(svc.install_programs(&["badhash".into()], false, false).is_err());
        assert!(svc.install_programs(&["future".into()], false, false).is_err());
        assert!(!paths.rules_dir.join("badhash.toml").exists());
        assert!(!paths.rules_dir.join("future.toml").exists());

        let _ = fs::remove_dir_all(paths.repos_dir.parent().unwrap());
    }

    #[test]
    fn test_update_installed_skips_locally_modified_without_force_and_backups_with_force() {
        let paths = temp_registry_paths("update_guard");
        let r1 = sample_rule("Foot", 1, "foot.tmpl");
        let t1 = sample_tmpl(1);
        let r2 = sample_rule("Foot", 2, "foot.tmpl");
        let t2 = sample_tmpl(2);

        let m1 = build_manifest(&[(
            "foot",
            "Foot",
            1,
            1,
            "1.6.0",
            &sha256_hex(r1.as_bytes()),
            &sha256_hex(t1.as_bytes()),
        )]);
        let m2 = build_manifest(&[(
            "foot",
            "Foot",
            2,
            2,
            "1.6.0",
            &sha256_hex(r2.as_bytes()),
            &sha256_hex(t2.as_bytes()),
        )]);

        let mut map1 = HashMap::new();
        map1.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(m1));
        map1.insert(format!("{OFFICIAL_REPO_URL}/rules/foot.toml"), Ok(r1));
        map1.insert(format!("{OFFICIAL_REPO_URL}/templates/foot.tmpl"), Ok(t1));

        let svc1 = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(map1),
            "1.6.0".into(),
        );
        svc1.ensure_default_repo_config().unwrap();
        svc1.sync_indexes().unwrap();
        svc1.install_programs(&["foot".into()], false, false).unwrap();

        // User edits local template manually
        fs::write(
            paths.templates_dir.join("foot.tmpl"),
            "# termonet-version: 1\n# user custom tweak\n",
        )
        .unwrap();

        let mut map2 = HashMap::new();
        map2.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(m2));
        map2.insert(format!("{OFFICIAL_REPO_URL}/rules/foot.toml"), Ok(r2.clone()));
        map2.insert(
            format!("{OFFICIAL_REPO_URL}/templates/foot.tmpl"),
            Ok(t2.clone()),
        );

        let svc2 = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(map2),
            "1.6.0".into(),
        );
        let rep = svc2.update_installed(false, true).unwrap();
        assert_eq!(rep.skipped_modified, vec!["foot".to_string()]);
        assert!(rep.updated.is_empty());

        // Forced update must backup and overwrite
        let rep_force = svc2.update_installed(true, true).unwrap();
        assert_eq!(rep_force.updated, vec!["foot".to_string()]);
        assert_eq!(
            fs::read_to_string(paths.templates_dir.join("foot.tmpl")).unwrap(),
            t2
        );
        assert!(paths.templates_dir.join("foot.tmpl.bak").exists());

        let _ = fs::remove_dir_all(paths.repos_dir.parent().unwrap());
    }

    #[test]
    fn test_catalog_pins_installed_repo_and_deduplicates_uninstalled_by_priority() {
        let paths = temp_registry_paths("catalog_pin");
        fs::create_dir_all(&paths.repos_dir).unwrap();
        fs::write(
            paths.repos_dir.join("custom.toml"),
            "name = \"custom\"\nurl = \"https://example.com/custom\"\npriority = 100\nenabled = true\n",
        )
        .unwrap();

        let r1 = sample_rule("Kitty", 1, "kitty.tmpl");
        let t1 = sample_tmpl(1);
        let r_custom = sample_rule("Kitty", 99, "kitty.tmpl");
        let r_foot_off = sample_rule("Foot", 1, "foot.tmpl");
        let t_foot_off = sample_tmpl(1);
        let r_foot_cust = sample_rule("Foot", 5, "foot.tmpl");
        let t_foot_cust = sample_tmpl(5);

        let m_official = build_manifest(&[
            (
                "kitty",
                "Kitty",
                1,
                1,
                "1.6.0",
                &sha256_hex(r1.as_bytes()),
                &sha256_hex(t1.as_bytes()),
            ),
            (
                "foot",
                "Foot",
                1,
                1,
                "1.6.0",
                &sha256_hex(r_foot_off.as_bytes()),
                &sha256_hex(t_foot_off.as_bytes()),
            ),
        ]);
        let m_custom = build_manifest(&[
            (
                "kitty",
                "Kitty Custom",
                99,
                1,
                "1.6.0",
                &sha256_hex(r_custom.as_bytes()),
                &sha256_hex(t1.as_bytes()),
            ),
            (
                "foot",
                "Foot Custom",
                5,
                5,
                "1.6.0",
                &sha256_hex(r_foot_cust.as_bytes()),
                &sha256_hex(t_foot_cust.as_bytes()),
            ),
        ]);

        let mut map = HashMap::new();
        map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(m_official));
        map.insert(format!("{OFFICIAL_REPO_URL}/rules/kitty.toml"), Ok(r1));
        map.insert(format!("{OFFICIAL_REPO_URL}/templates/kitty.tmpl"), Ok(t1));
        map.insert("https://example.com/custom/registry.toml".into(), Ok(m_custom));

        let svc = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(map),
            "1.6.0".into(),
        );
        svc.ensure_default_repo_config().unwrap();
        svc.sync_indexes().unwrap();
        svc.install_programs(&["official/kitty".into()], false, false)
            .unwrap();

        let (catalog, _warnings) = svc.catalog().unwrap();
        let kitty = catalog.iter().find(|e| e.id == "kitty").unwrap();
        assert_eq!(
            kitty.status,
            CatalogStatus::Installed {
                repo: "official".into(),
                update_available: None,
                locally_modified: false,
            }
        );

        let foot = catalog.iter().find(|e| e.id == "foot").unwrap();
        assert_eq!(
            foot.status,
            CatalogStatus::Available {
                repo: "custom".into(),
                compatible: true,
            }
        );

        let _ = fs::remove_dir_all(paths.repos_dir.parent().unwrap());
    }

    #[test]
    fn test_malformed_custom_repo_produces_warning_without_breaking_official() {
        let paths = temp_registry_paths("bad_custom_repo");
        fs::create_dir_all(&paths.repos_dir).unwrap();
        fs::write(paths.repos_dir.join("broken.toml"), "not valid = [toml").unwrap();

        let r1 = sample_rule("Kitty", 1, "kitty.tmpl");
        let t1 = sample_tmpl(1);
        let manifest = build_manifest(&[(
            "kitty",
            "Kitty",
            1,
            1,
            "1.6.0",
            &sha256_hex(r1.as_bytes()),
            &sha256_hex(t1.as_bytes()),
        )]);
        let mut map = HashMap::new();
        map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(manifest));

        let svc = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(map),
            "1.6.0".into(),
        );
        svc.ensure_default_repo_config().unwrap();
        let (repos, repo_warnings) = svc.load_repos().unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, OFFICIAL_REPO_NAME);
        assert!(!repo_warnings.is_empty());

        let sync_warnings = svc.sync_indexes().unwrap();
        assert!(!sync_warnings.is_empty());
        let (catalog, _) = svc.catalog().unwrap();
        assert!(catalog.iter().any(|c| c.id == "kitty"));

        let _ = fs::remove_dir_all(paths.repos_dir.parent().unwrap());
    }

    #[test]
    fn test_maybe_auto_update_respects_7_day_ttl_and_returns_warning_on_network_error() {
        let paths = temp_registry_paths("auto_ttl");
        let r1 = sample_rule("Kitty", 1, "kitty.tmpl");
        let t1 = sample_tmpl(1);
        let manifest = build_manifest(&[(
            "kitty",
            "Kitty",
            1,
            1,
            "1.6.0",
            &sha256_hex(r1.as_bytes()),
            &sha256_hex(t1.as_bytes()),
        )]);
        let mut map = HashMap::new();
        map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(manifest));
        map.insert(format!("{OFFICIAL_REPO_URL}/rules/kitty.toml"), Ok(r1));
        map.insert(format!("{OFFICIAL_REPO_URL}/templates/kitty.tmpl"), Ok(t1));

        let svc = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(map),
            "1.6.0".into(),
        );
        svc.ensure_default_repo_config().unwrap();
        svc.sync_indexes().unwrap();
        svc.install_programs(&["kitty".into()], false, false).unwrap();

        let state = AppStateCache::load_from_path(&paths.state_file).unwrap();
        let now = state.last_registry_sync_at.unwrap();

        let cfg_disabled = AppConfig {
            auto_update_rules: false,
            ..Default::default()
        };
        assert_eq!(
            svc.maybe_auto_update(&cfg_disabled, now + WEEKLY_SYNC_INTERVAL_SECS + 10),
            AutoUpdateOutcome::SkippedDisabled
        );

        let cfg = AppConfig {
            auto_update_rules: true,
            ..Default::default()
        };
        assert_eq!(
            svc.maybe_auto_update(&cfg, now + 3600),
            AutoUpdateOutcome::SkippedIntervalNotElapsed
        );

        // Network failure after 7 days must be non-fatal
        let mut failing_map = HashMap::new();
        failing_map.insert(
            format!("{OFFICIAL_REPO_URL}/registry.toml"),
            Err("offline".into()),
        );
        let svc_offline = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(failing_map),
            "1.6.0".into(),
        );
        match svc_offline.maybe_auto_update(&cfg, now + WEEKLY_SYNC_INTERVAL_SECS + 1) {
            AutoUpdateOutcome::FailedNonFatal(msg) => assert!(msg.contains("offline")),
            other => panic!("Expected non-fatal network failure, got {other:?}"),
        }

        let _ = fs::remove_dir_all(paths.repos_dir.parent().unwrap());
    }

    #[test]
    fn test_remove_program_deletes_tracked_and_refuses_local_rule() {
        let paths = temp_registry_paths("remove");
        let r1 = sample_rule("Kitty", 1, "kitty.tmpl");
        let t1 = sample_tmpl(1);
        let manifest = build_manifest(&[(
            "kitty",
            "Kitty",
            1,
            1,
            "1.6.0",
            &sha256_hex(r1.as_bytes()),
            &sha256_hex(t1.as_bytes()),
        )]);
        let mut map = HashMap::new();
        map.insert(format!("{OFFICIAL_REPO_URL}/registry.toml"), Ok(manifest));
        map.insert(format!("{OFFICIAL_REPO_URL}/rules/kitty.toml"), Ok(r1));
        map.insert(format!("{OFFICIAL_REPO_URL}/templates/kitty.tmpl"), Ok(t1));

        let svc = RegistryService::with_paths_and_transport(
            paths.clone(),
            MockTransport::new(map),
            "1.6.0".into(),
        );
        svc.ensure_default_repo_config().unwrap();
        svc.sync_indexes().unwrap();
        svc.install_programs(&["kitty".into()], false, false).unwrap();

        fs::write(
            paths.rules_dir.join("mycustom.toml"),
            sample_rule("MyCustom", 1, "mycustom.tmpl"),
        )
        .unwrap();

        assert!(svc.remove_program("mycustom").is_err());
        assert!(paths.rules_dir.join("mycustom.toml").exists());

        svc.remove_program("kitty").unwrap();
        assert!(!paths.rules_dir.join("kitty.toml").exists());
        assert!(!paths.templates_dir.join("kitty.tmpl").exists());

        let _ = fs::remove_dir_all(paths.repos_dir.parent().unwrap());
    }
}
