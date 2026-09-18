//! Marketplace service: registry + installer + plugin host orchestration.
//!
//! This is the UI backend. Reads (`catalog`, `installed`, `updates`) never
//! touch the network except through the registry client's offline fallback;
//! writes (`install`, `update`, `rollback`, `uninstall`) verify before they
//! mutate and keep the running host consistent (stop → swap → restart, with
//! automatic rollback when the new build fails to start).

use crate::client::{RegistryClient, ResolvedIndex};
use crate::installer::{
    ExpectedPackage, ExtensionKind, InstallReport, InstalledEntry, MarketplacePaths,
    RollbackReport, UpdateInfo,
};
use crate::{installer, RegistryError, Result};
use sonora_plugin::PluginHost;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// UI-facing catalog: index entries merged with install state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CatalogEntry {
    pub kind: ExtensionKind,
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub author_url: Option<String>,
    pub category: String,
    pub homepage: Option<String>,
    pub capabilities: Vec<String>,
    pub latest_version: String,
    pub min_sonora_version: String,
    pub installed_version: Option<String>,
    pub update_available: Option<String>,
}

/// Full catalog response for the marketplace UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Catalog {
    pub plugins: Vec<CatalogEntry>,
    pub themes: Vec<CatalogEntry>,
    pub offline: bool,
    pub stale: bool,
    pub fetched_at: u64,
    pub registry_url: String,
}

pub struct Marketplace {
    paths: MarketplacePaths,
    client: RegistryClient,
    host: Arc<PluginHost>,
}

impl Marketplace {
    /// Build from a data dir. `registry_url`: explicit override, else
    /// `SONORA_REGISTRY_URL`, else the default. `transport`: override for
    /// tests; otherwise the production transport.
    pub fn new(
        data_dir: &Path,
        host: Arc<PluginHost>,
        registry_url: Option<String>,
        transport: Option<Arc<dyn sonora_plugin::NetworkTransport>>,
    ) -> Result<Self> {
        // No directories are created here: every mutating operation ensures
        // its own dirs, so read-only use (and in-memory tests) stay side-effect free.
        let paths = MarketplacePaths::new(data_dir);
        let url = registry_url
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(crate::client::default_registry_url);
        let transport =
            transport.unwrap_or_else(|| Arc::new(sonora_plugin::RealNetworkTransport::new()));
        let client = RegistryClient::new(url, transport, paths.cache_dir.clone())?;
        Ok(Self {
            paths,
            client,
            host,
        })
    }

    pub fn registry_url(&self) -> &str {
        self.client.base_url()
    }

    /// Replace the download transport (tests inject fakes).
    pub fn set_transport(&self, transport: Arc<dyn sonora_plugin::NetworkTransport>) {
        self.client.set_transport(transport);
    }

    /// Refresh the index from the network (errors when offline with no cache
    /// to preserve — use [`Marketplace::catalog`] for graceful fallback).
    pub fn refresh(&self) -> Result<Catalog> {
        let resolved = self.client.refresh()?;
        Ok(self.merge(resolved))
    }

    /// Catalog with offline fallback: network first, last good cache when the
    /// network fails. Only errors with no cache at all.
    pub fn catalog(&self) -> Result<Catalog> {
        let resolved = self.client.resolve()?;
        Ok(self.merge(resolved))
    }

    fn merge(&self, resolved: ResolvedIndex) -> Catalog {
        let receipts = installer::installed_receipts(&self.paths).unwrap_or_default();
        let update_list = installer::check_updates(
            &resolved.plugins,
            &resolved.themes,
            &receipts_to_map(&receipts),
        );
        let by_id: HashMap<&str, &UpdateInfo> =
            update_list.iter().map(|u| (u.id.as_str(), u)).collect();

        let plugins = resolved
            .plugins
            .iter()
            .map(|p| {
                let installed = receipts.get(p.id()).map(|r| r.version.clone());
                CatalogEntry {
                    kind: ExtensionKind::Plugin,
                    id: p.id().to_string(),
                    name: p.name().to_string(),
                    description: p.description().to_string(),
                    author: p.author_name().to_string(),
                    author_url: p.author_url().map(str::to_string),
                    category: p.category().to_string(),
                    homepage: p.homepage().map(str::to_string),
                    capabilities: p.capabilities().to_vec(),
                    latest_version: p.latest_version().to_string(),
                    min_sonora_version: p.min_sonora_version().to_string(),
                    installed_version: installed,
                    update_available: by_id.get(p.id()).map(|u| u.available.clone()),
                }
            })
            .collect();
        let themes = resolved
            .themes
            .iter()
            .map(|t| {
                let installed = receipts.get(t.id()).map(|r| r.version.clone());
                CatalogEntry {
                    kind: ExtensionKind::Theme,
                    id: t.id().to_string(),
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    author: t.author_name().to_string(),
                    author_url: t.author_url().map(str::to_string),
                    category: t.category().to_string(),
                    homepage: t.homepage().map(str::to_string),
                    capabilities: Vec::new(),
                    latest_version: t.latest_version().to_string(),
                    min_sonora_version: t.min_sonora_version().to_string(),
                    installed_version: installed,
                    update_available: by_id.get(t.id()).map(|u| u.available.clone()),
                }
            })
            .collect();
        Catalog {
            plugins,
            themes,
            offline: resolved.offline,
            stale: resolved.stale,
            fetched_at: resolved.fetched_at,
            registry_url: self.client.base_url().to_string(),
        }
    }

    /// Installed extensions merged with live host state.
    pub fn installed(&self) -> Result<Vec<InstalledEntry>> {
        let receipts = installer::installed_receipts(&self.paths)?;
        let active = self.active_theme().unwrap_or(None);
        let live: HashMap<String, String> = self
            .host
            .list()
            .into_iter()
            .map(|p| (p.id, p.state))
            .collect();
        let mut out = Vec::new();
        for (id, receipt) in &receipts {
            let (name, capabilities, state) = match receipt.kind {
                ExtensionKind::Plugin => {
                    // Dir names use raw ids: the validated id charset is
                    // filesystem-safe, and sanitizing would corrupt lookups.
                    let dir = self.paths.plugins_dir.join(id);
                    let (name, caps) =
                        read_installed_plugin_meta(&dir).unwrap_or_else(|| (id.clone(), vec![]));
                    let state = live
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| "inactive".to_string());
                    (name, caps, state)
                }
                ExtensionKind::Theme => {
                    let dir = self.paths.themes_dir.join(id);
                    match read_installed_theme(&dir) {
                        Some(def) => {
                            let state = if active.as_deref() == Some(id.as_str()) {
                                "active"
                            } else {
                                "installed"
                            };
                            (def.name.clone(), vec![], state.to_string())
                        }
                        // Receipt exists but files are corrupt/missing: surface
                        // as invalid so the UI offers removal. Never crash.
                        None => (id.clone(), vec![], "invalid".to_string()),
                    }
                }
            };
            out.push(InstalledEntry {
                id: id.clone(),
                kind: receipt.kind,
                name,
                version: receipt.version.clone(),
                state,
                capabilities,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Available updates for installed extensions.
    pub fn updates(&self) -> Result<Vec<UpdateInfo>> {
        let resolved = self.client.resolve()?;
        let receipts = installer::installed_receipts(&self.paths)?;
        Ok(installer::check_updates(
            &resolved.plugins,
            &resolved.themes,
            &receipts_to_map(&receipts),
        ))
    }

    /// Install a catalog extension at `version` (default: newest compatible).
    /// Refuses incompatible versions; never executes unverified bytes.
    pub fn install(&self, id: &str, version: Option<&str>) -> Result<InstallReport> {
        self.paths.ensure_dirs()?;
        let resolved = self.client.resolve()?;
        // Plugin or theme?
        if let Some(entry) = resolved.find_plugin(id) {
            let target = self.pick_version_plugin(entry, version)?;
            if self.is_installed_at(id, &target.version) {
                return Err(RegistryError::AlreadyInstalled(format!(
                    "{id} {}",
                    target.version
                )));
            }
            let bytes = installer::download_verified(
                &self.transport(),
                &target.download_url,
                &target.sha256,
                crate::installer::PACKAGE_MAX_BYTES,
            )
            .map_err(|e| self.relabel_download(id, e))?;
            let expected = ExpectedPackage {
                id: entry.id(),
                version: &target.version,
                capabilities: entry.capabilities(),
            };
            installer::install_plugin_bytes(
                &self.paths,
                &self.host,
                &expected,
                &bytes,
                &target.sha256,
                self.client.base_url(),
            )
        } else if let Some(entry) = resolved.find_theme(id) {
            let target = self.pick_version_theme(entry, version)?;
            if self.is_installed_at(id, &target.version) {
                return Err(RegistryError::AlreadyInstalled(format!(
                    "{id} {}",
                    target.version
                )));
            }
            let bytes = installer::download_verified(
                &self.transport(),
                &target.download_url,
                &target.sha256,
                crate::installer::PACKAGE_MAX_BYTES,
            )
            .map_err(|e| self.relabel_download(id, e))?;
            installer::install_theme_bytes(
                &self.paths,
                entry.id(),
                &target.version,
                &bytes,
                &target.sha256,
                self.client.base_url(),
            )
        } else {
            Err(RegistryError::NotInstalled(format!(
                "{id} is not in the registry"
            )))
        }
    }

    /// Update to the newest compatible version. On start failure the previous
    /// version is restored automatically.
    pub fn update(&self, id: &str) -> Result<InstallReport> {
        let updates = self.updates()?;
        let info = updates
            .into_iter()
            .find(|u| u.id == id)
            .ok_or_else(|| RegistryError::NoUpdateAvailable(id.to_string()))?;
        if !info.compatible {
            return Err(RegistryError::NoCompatibleVersion(
                id.to_string(),
                format!(
                    "newest is {} but requires {}",
                    info.available, info.min_sonora_version
                ),
            ));
        }
        let report = self.install(id, Some(&info.available))?;
        if !report.started && report.kind == ExtensionKind::Plugin {
            // New build installed but won't start: roll back automatically.
            let _ = installer::rollback(&self.paths, &self.host, id, None);
            return Err(RegistryError::Transport(format!(
                "update installed but failed to start; rolled back: {}",
                report.start_error.unwrap_or_default()
            )));
        }
        Ok(report)
    }

    pub fn rollback(&self, id: &str, version: Option<&str>) -> Result<RollbackReport> {
        self.paths.ensure_dirs()?;
        installer::rollback(&self.paths, &self.host, id, version)
    }

    pub fn uninstall(&self, id: &str) -> Result<()> {
        self.paths.ensure_dirs()?;
        installer::uninstall(&self.paths, &self.host, id)?;
        // Uninstalling the active theme must not leave a dangling selection.
        if self.active_theme().unwrap_or(None).as_deref() == Some(id) {
            self.set_active_theme(None)?;
        }
        Ok(())
    }

    // -- first-class themes ----------------------------------------------------

    /// All installed, valid themes for the theme selector. Malformed installs
    /// are skipped (with a warning) instead of failing the listing.
    pub fn installed_themes(&self) -> Result<Vec<InstalledTheme>> {
        let active = self.active_theme().unwrap_or(None);
        let mut out = Vec::new();
        let entries = std::fs::read_dir(&self.paths.themes_dir)
            .map(|rd| rd.filter_map(std::result::Result::ok).collect::<Vec<_>>())
            .unwrap_or_default();
        for entry in entries {
            let dir = entry.path();
            if !dir.is_dir()
                || dir
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(true)
            {
                continue;
            }
            let text = match std::fs::read_to_string(dir.join("theme.json")) {
                Ok(t) => t,
                Err(_) => continue,
            };
            match crate::theme::validate_theme_definition(&text) {
                Ok(def) => {
                    let has_css = dir.join("theme.css").is_file();
                    out.push(InstalledTheme {
                        active: active.as_deref() == Some(def.id.as_str()),
                        id: def.id.clone(),
                        name: def.name.clone(),
                        version: def.version.to_string(),
                        mode: def.mode.as_str().to_string(),
                        description: def.description.clone(),
                        author: def.author.clone(),
                        definition: def,
                        has_css,
                    });
                }
                Err(e) => {
                    tracing::warn!("skipping invalid installed theme at {}: {e}", dir.display());
                }
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Validated full definition for one installed theme (feeds the frontend
    /// theme engine, which re-validates against its own schema).
    pub fn theme_definition(&self, id: &str) -> Result<crate::theme::ThemeDefinition> {
        let text = std::fs::read_to_string(self.paths.themes_dir.join(id).join("theme.json"))
            .map_err(|_| RegistryError::NotInstalled(format!("theme '{id}' is not installed")))?;
        crate::theme::validate_theme_definition(&text)
    }

    /// Supplemental stylesheet for one installed theme, re-validated at serve
    /// time (installed files are not trusted). `None` when the theme ships no
    /// `theme.css`.
    pub fn theme_css(&self, id: &str) -> Result<Option<String>> {
        let path = self.paths.themes_dir.join(id).join("theme.css");
        if !path.is_file() {
            // The theme itself must still be valid, or report it missing.
            self.theme_definition(id)?;
            return Ok(None);
        }
        let bytes = std::fs::read(&path).map_err(|e| RegistryError::Io(e.to_string()))?;
        crate::theme::validate_theme_css(id, &bytes)?;
        String::from_utf8(bytes).map(Some).map_err(|_| {
            RegistryError::UnsafePackage(id.to_string(), "theme.css must be UTF-8".to_string())
        })
    }

    /// Persisted marketplace theme selection (`None` = follow the frontend
    /// built-in default). Only installed, valid themes are accepted.
    pub fn active_theme(&self) -> Result<Option<String>> {
        match std::fs::read_to_string(self.active_theme_path()) {
            Ok(text) => {
                let value: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| RegistryError::Io(format!("active theme file corrupt: {e}")))?;
                Ok(value
                    .get("theme_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(RegistryError::Io(e.to_string())),
        }
    }

    /// Persist a marketplace theme selection. `None` clears it (reset to
    /// built-in). Unknown or invalid theme ids are rejected.
    pub fn set_active_theme(&self, id: Option<&str>) -> Result<()> {
        if let Some(id) = id {
            if id.trim().is_empty() || id.len() > 128 {
                return Err(RegistryError::NotInstalled("empty theme id".to_string()));
            }
            // Must be installed AND valid — never persist a dangling selection.
            self.theme_definition(id)?;
        }
        if let Some(parent) = self.active_theme_path().parent() {
            std::fs::create_dir_all(parent).map_err(|e| RegistryError::Io(e.to_string()))?;
        }
        let doc = serde_json::json!({ "theme_id": id });
        crate::atomic_write(
            &self.active_theme_path(),
            serde_json::to_string(&doc)
                .map_err(|e| RegistryError::Io(e.to_string()))?
                .as_bytes(),
        )
    }

    fn active_theme_path(&self) -> std::path::PathBuf {
        self.paths
            .cache_dir
            .parent()
            .map(|p| p.join("active-theme.json"))
            .unwrap_or_else(|| self.paths.cache_dir.join("active-theme.json"))
    }

    fn transport(&self) -> Arc<dyn sonora_plugin::NetworkTransport> {
        self.client.transport()
    }

    fn pick_version_plugin(
        &self,
        entry: &crate::schema::RegistryPlugin,
        requested: Option<&str>,
    ) -> Result<PickedVersion> {
        match requested {
            Some(v) => {
                let target = entry.version(v).ok_or_else(|| {
                    RegistryError::NoCompatibleVersion(
                        entry.id().to_string(),
                        format!("unknown version {v}"),
                    )
                })?;
                self.ensure_compatible_plugin(entry, target)?;
                Ok(PickedVersion {
                    version: target.version.clone(),
                    download_url: target.download_url.clone(),
                    sha256: target.sha256.clone(),
                })
            }
            None => self.newest_compatible_plugin(entry).ok_or_else(|| {
                RegistryError::NoCompatibleVersion(
                    entry.id().to_string(),
                    "no compatible version published".to_string(),
                )
            }),
        }
    }

    fn pick_version_theme(
        &self,
        entry: &crate::schema::RegistryTheme,
        requested: Option<&str>,
    ) -> Result<PickedVersion> {
        match requested {
            Some(v) => {
                let target = entry.version(v).ok_or_else(|| {
                    RegistryError::NoCompatibleVersion(
                        entry.id().to_string(),
                        format!("unknown version {v}"),
                    )
                })?;
                self.ensure_sonora_compatible(entry.min_sonora_version(), entry.id())?;
                Ok(PickedVersion {
                    version: target.version.clone(),
                    download_url: target.download_url.clone(),
                    sha256: target.sha256.clone(),
                })
            }
            None => {
                let mut cands: Vec<&crate::schema::RegistryVersion> =
                    entry.versions().iter().collect();
                cands.sort_by(|a, b| version_key(&b.version).cmp(&version_key(&a.version)));
                cands
                    .into_iter()
                    .find(|_| {
                        self.ensure_sonora_compatible(entry.min_sonora_version(), entry.id())
                            .is_ok()
                    })
                    .map(|t| PickedVersion {
                        version: t.version.clone(),
                        download_url: t.download_url.clone(),
                        sha256: t.sha256.clone(),
                    })
                    .ok_or_else(|| {
                        RegistryError::NoCompatibleVersion(
                            entry.id().to_string(),
                            "no compatible version published".to_string(),
                        )
                    })
            }
        }
    }

    fn newest_compatible_plugin(
        &self,
        entry: &crate::schema::RegistryPlugin,
    ) -> Option<PickedVersion> {
        if entry.api_version() != sonora_plugin::PLUGIN_API_VERSION {
            return None;
        }
        if self
            .ensure_sonora_compatible(entry.min_sonora_version(), entry.id())
            .is_err()
        {
            return None;
        }
        let mut cands: Vec<&crate::schema::RegistryVersion> = entry.versions().iter().collect();
        cands.sort_by(|a, b| version_key(&b.version).cmp(&version_key(&a.version)));
        cands.into_iter().next().map(|t| PickedVersion {
            version: t.version.clone(),
            download_url: t.download_url.clone(),
            sha256: t.sha256.clone(),
        })
    }

    fn ensure_compatible_plugin(
        &self,
        entry: &crate::schema::RegistryPlugin,
        target: &crate::schema::RegistryVersion,
    ) -> Result<()> {
        if entry.api_version() != sonora_plugin::PLUGIN_API_VERSION {
            return Err(RegistryError::NoCompatibleVersion(
                entry.id().to_string(),
                format!(
                    "{} needs plugin API v{}",
                    target.version,
                    entry.api_version()
                ),
            ));
        }
        self.ensure_sonora_compatible(entry.min_sonora_version(), entry.id())
    }

    fn ensure_sonora_compatible(&self, min: &str, id: &str) -> Result<()> {
        let min_v = semver::Version::parse(min).map_err(|_| {
            RegistryError::NoCompatibleVersion(id.to_string(), "bad min_sonora_version".to_string())
        })?;
        let cur = semver::Version::parse(crate::SONORA_VERSION)
            .map_err(|e| RegistryError::Transport(format!("bad client version: {e}")))?;
        if cur < min_v {
            return Err(RegistryError::NoCompatibleVersion(
                id.to_string(),
                format!("requires Sonora {min} (running {})", crate::SONORA_VERSION),
            ));
        }
        Ok(())
    }

    fn is_installed_at(&self, id: &str, version: &str) -> bool {
        installer::installed_receipts(&self.paths)
            .ok()
            .and_then(|m| m.get(id).cloned())
            .map(|r| r.version == version)
            .unwrap_or(false)
    }

    fn relabel_download(&self, id: &str, e: RegistryError) -> RegistryError {
        match e {
            RegistryError::ChecksumMismatch {
                expected, actual, ..
            } => RegistryError::ChecksumMismatch {
                id: id.to_string(),
                version: String::new(),
                expected,
                actual,
            },
            other => other,
        }
    }
}

struct PickedVersion {
    version: String,
    download_url: String,
    sha256: String,
}

fn version_key(v: &str) -> (u64, u64, u64) {
    semver::Version::parse(v)
        .map(|p| (p.major, p.minor, p.patch))
        .unwrap_or((0, 0, 0))
}

fn receipts_to_map(
    receipts: &std::collections::BTreeMap<String, installer::Receipt>,
) -> HashMap<String, installer::Receipt> {
    receipts
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// A validated installed theme for the theme selector UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledTheme {
    pub id: String,
    pub name: String,
    pub version: String,
    pub mode: String,
    pub description: Option<String>,
    pub author: String,
    pub definition: crate::theme::ThemeDefinition,
    pub has_css: bool,
    pub active: bool,
}

fn read_installed_theme(dir: &Path) -> Option<crate::theme::ThemeDefinition> {
    let text = std::fs::read_to_string(dir.join("theme.json")).ok()?;
    crate::theme::validate_theme_definition(&text).ok()
}

fn read_installed_plugin_meta(dir: &Path) -> Option<(String, Vec<String>)> {
    let text = std::fs::read_to_string(dir.join("manifest.json")).ok()?;
    let manifest = sonora_plugin::PluginManifest::from_json(&text).ok()?;
    Some((
        manifest.name.clone(),
        manifest
            .capabilities
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
    ))
}
