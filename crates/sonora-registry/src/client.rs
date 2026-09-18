//! Registry index client: fetch, validate, cache, offline fallback.
//!
//! `RegistryClient::resolve()` is the normal entry point: it tries a network
//! refresh and falls back to the last validated cache when offline, reporting
//! which path served the data. Nothing unverified is ever cached or returned.

use crate::schema::{RegistryIndex, RegistryPlugin, RegistryTheme};
use crate::{atomic_write, now_secs, RegistryError, Result};
use sonora_plugin::{FakeNetworkTransport, NetworkTransport};
use std::path::PathBuf;
use std::sync::Arc;

/// Default registry base URL (mirrored static hosting; override with
/// `SONORA_REGISTRY_URL` for private registries and tests).
pub const DEFAULT_REGISTRY_URL: &str = "https://registry.sonora.audio/v1";

/// [Config] Cache freshness horizon (seconds).
pub const REGISTRY_CACHE_TTL_SECS: u64 = 24 * 3600;

/// Upper bound for a single index document (DoS guard before parsing).
pub const REGISTRY_MAX_INDEX_BYTES: usize = 2 * 1024 * 1024;

/// Registry base URL, honoring the environment override.
pub fn default_registry_url() -> String {
    std::env::var("SONORA_REGISTRY_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_REGISTRY_URL.to_string())
}

/// A validated index plus provenance for the UI (offline/stale banners).
#[derive(Debug, Clone)]
pub struct ResolvedIndex {
    pub plugins: Vec<RegistryPlugin>,
    pub themes: Vec<RegistryTheme>,
    /// `true` when served from cache because the network failed.
    pub offline: bool,
    /// `true` when the data (cache) is older than the TTL.
    pub stale: bool,
    /// Unix seconds when this data was fetched (network) or cached.
    pub fetched_at: u64,
}

impl ResolvedIndex {
    pub fn find_plugin(&self, id: &str) -> Option<&RegistryPlugin> {
        self.plugins.iter().find(|p| p.id() == id)
    }
    pub fn find_theme(&self, id: &str) -> Option<&RegistryTheme> {
        self.themes.iter().find(|t| t.id() == id)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheMeta {
    fetched_at: u64,
    base_url: String,
}

/// Fetches and caches `plugins.json` / `themes.json`.
pub struct RegistryClient {
    base_url: String,
    transport: std::sync::Mutex<Arc<dyn NetworkTransport>>,
    cache_dir: PathBuf,
}

impl RegistryClient {
    pub fn new(
        base_url: String,
        transport: Arc<dyn NetworkTransport>,
        cache_dir: PathBuf,
    ) -> Result<Self> {
        let base = base_url.trim_end_matches('/').to_string();
        if !base.starts_with("https://") {
            return Err(RegistryError::InvalidIndex(format!(
                "registry base URL must use https://: {base}"
            )));
        }
        Ok(Self {
            base_url: base,
            transport: std::sync::Mutex::new(transport),
            cache_dir,
        })
    }

    /// Client with the default (or environment-overridden) registry URL and
    /// the production transport.
    pub fn with_defaults(cache_dir: PathBuf) -> Result<Self> {
        Self::new(
            default_registry_url(),
            Arc::new(sonora_plugin::RealNetworkTransport::new()),
            cache_dir,
        )
    }

    /// Test client with a scripted transport.
    pub fn for_tests(
        base_url: &str,
        fake: Arc<FakeNetworkTransport>,
        cache_dir: PathBuf,
    ) -> Result<Self> {
        Self::new(base_url.to_string(), fake, cache_dir)
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The transport this client downloads through (shared with installs so
    /// tests inject fakes once).
    pub fn transport(&self) -> Arc<dyn NetworkTransport> {
        self.transport
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Replace the transport (tests inject [`FakeNetworkTransport`]).
    pub fn set_transport(&self, transport: Arc<dyn NetworkTransport>) {
        *self.transport.lock().unwrap_or_else(|e| e.into_inner()) = transport;
    }

    fn plugins_url(&self) -> String {
        format!("{}/plugins.json", self.base_url)
    }

    fn themes_url(&self) -> String {
        format!("{}/themes.json", self.base_url)
    }

    fn plugins_cache(&self) -> PathBuf {
        self.cache_dir.join("plugins.json")
    }

    fn themes_cache(&self) -> PathBuf {
        self.cache_dir.join("themes.json")
    }

    fn meta_cache(&self) -> PathBuf {
        self.cache_dir.join("meta.json")
    }

    /// Force a network refresh. Both documents are validated **before**
    /// anything is written; the cache is only replaced on full success.
    pub fn refresh(&self) -> Result<ResolvedIndex> {
        let plugins_text = self.fetch_text(&self.plugins_url(), true)?;
        let plugins = RegistryIndex::plugins_from_json(&plugins_text)?;
        // themes.json is optional: a 404 means "no themes published yet".
        let themes = match self.fetch_text(&self.themes_url(), false) {
            Ok(text) => RegistryIndex::themes_from_json(&text)?,
            Err(RegistryError::Transport(_)) => Vec::new(),
            Err(e) => return Err(e),
        };
        let fetched_at = now_secs();
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| RegistryError::Io(e.to_string()))?;
        atomic_write(&self.plugins_cache(), plugins_text.as_bytes())?;
        // Cache the validated themes document in canonical form.
        let themes_doc = serde_json::json!({
            "schema": crate::schema::REGISTRY_SCHEMA_VERSION,
            "themes": themes,
        });
        atomic_write(
            &self.themes_cache(),
            serde_json::to_string(&themes_doc)
                .map_err(|e| RegistryError::Io(e.to_string()))?
                .as_bytes(),
        )?;
        let meta = CacheMeta {
            fetched_at,
            base_url: self.base_url.clone(),
        };
        atomic_write(
            &self.meta_cache(),
            serde_json::to_string(&meta)
                .map_err(|e| RegistryError::Io(e.to_string()))?
                .as_bytes(),
        )?;
        Ok(ResolvedIndex {
            plugins,
            themes,
            offline: false,
            stale: false,
            fetched_at,
        })
    }

    /// Read the last validated cache, if any.
    pub fn cached(&self) -> Result<Option<ResolvedIndex>> {
        let plugins_path = self.plugins_cache();
        if !plugins_path.is_file() {
            return Ok(None);
        }
        let plugins_text =
            std::fs::read_to_string(&plugins_path).map_err(|e| RegistryError::Io(e.to_string()))?;
        let plugins = RegistryIndex::plugins_from_json(&plugins_text)?;
        let themes = match std::fs::read_to_string(self.themes_cache()) {
            Ok(text) => RegistryIndex::themes_from_json(&text)?,
            Err(_) => Vec::new(),
        };
        let fetched_at = std::fs::read_to_string(self.meta_cache())
            .ok()
            .and_then(|t| serde_json::from_str::<CacheMeta>(&t).ok())
            .map(|m| m.fetched_at)
            .unwrap_or(0);
        let stale = now_secs().saturating_sub(fetched_at) > REGISTRY_CACHE_TTL_SECS;
        Ok(Some(ResolvedIndex {
            plugins,
            themes,
            offline: true,
            stale,
            fetched_at,
        }))
    }

    /// Normal entry point: refresh, falling back to cache when offline.
    /// Errors only when the network fails **and** no cache exists.
    pub fn resolve(&self) -> Result<ResolvedIndex> {
        match self.refresh() {
            Ok(resolved) => Ok(resolved),
            Err(RegistryError::Transport(_)) => self.cached()?.ok_or(RegistryError::Offline),
            Err(e) => Err(e),
        }
    }

    fn fetch_text(&self, url: &str, required: bool) -> Result<String> {
        if url.len() > 2048 {
            return Err(RegistryError::InvalidIndex(
                "registry URL too long".to_string(),
            ));
        }
        let transport = self.transport();
        let bytes = transport
            .fetch(url, REGISTRY_MAX_INDEX_BYTES)
            .map_err(|e| match e {
                sonora_plugin::NetworkError::HttpStatus(404) if !required => {
                    RegistryError::Transport("not found".to_string())
                }
                other => RegistryError::Transport(format!("{other:?}")),
            })?;
        String::from_utf8(bytes)
            .map_err(|_| RegistryError::InvalidIndex("index is not UTF-8".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sonora-regtest-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn plugins_doc() -> String {
        serde_json::json!({
            "schema": 1,
            "plugins": [{
                "id": "org.example.cached",
                "name": "Cached",
                "description": "Cached entry.",
                "author": {"name": "T"},
                "category": "utility",
                "capabilities": ["storage:cache"],
                "api_version": 1,
                "min_sonora_version": "0.1.0",
                "latest_version": "1.0.0",
                "versions": [{
                    "version": "1.0.0",
                    "download_url": "https://example.com/c.zip",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "changelog": "First."
                }]
            }]
        })
        .to_string()
    }

    #[test]
    fn refresh_caches_and_resolve_reads_cache() {
        let dir = test_dir("refresh");
        let fake = Arc::new(FakeNetworkTransport::new());
        fake.route(
            "plugins.json",
            sonora_plugin::FakeOutcome::Status(200, plugins_doc().into_bytes()),
        );
        fake.route(
            "themes.json",
            sonora_plugin::FakeOutcome::Status(404, vec![]),
        );
        let client =
            RegistryClient::for_tests("https://registry.test/v1", fake, dir.join("cache")).unwrap();
        let resolved = client.resolve().unwrap();
        assert!(!resolved.offline);
        assert_eq!(resolved.plugins.len(), 1);
        assert!(resolved.themes.is_empty());
        assert!(dir.join("cache/plugins.json").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn offline_falls_back_to_cache() {
        let dir = test_dir("offline");
        let fake = Arc::new(FakeNetworkTransport::new());
        fake.route(
            "plugins.json",
            sonora_plugin::FakeOutcome::Status(200, plugins_doc().into_bytes()),
        );
        fake.route(
            "themes.json",
            sonora_plugin::FakeOutcome::Status(404, vec![]),
        );
        let client =
            RegistryClient::for_tests("https://registry.test/v1", fake.clone(), dir.join("cache"))
                .unwrap();
        client.refresh().unwrap();
        // Now break the network: resolve must serve the cache, flagged offline.
        fake.route(
            "plugins.json",
            sonora_plugin::FakeOutcome::Io("down".to_string()),
        );
        let resolved = client.resolve().unwrap();
        assert!(resolved.offline);
        assert_eq!(resolved.plugins.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn offline_without_cache_errors() {
        let dir = test_dir("nocache");
        let fake = Arc::new(FakeNetworkTransport::new());
        let client =
            RegistryClient::for_tests("https://registry.test/v1", fake, dir.join("cache")).unwrap();
        assert!(matches!(client.resolve(), Err(RegistryError::Offline)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_non_https_base() {
        let fake = Arc::new(FakeNetworkTransport::new());
        assert!(
            RegistryClient::for_tests("http://registry.test/v1", fake, PathBuf::from("/tmp"))
                .is_err()
        );
    }
}
