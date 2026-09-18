//! Registry index format (`v1/plugins.json`, `v1/themes.json`) and validation.
//!
//! The index is authored in git, mirrored to static hosting, and consumed as
//! **untrusted input**: every field is validated before use. Unknown extra
//! fields are ignored so the schema can grow without breaking old clients.

use serde::{Deserialize, Serialize};
use sonora_plugin::{validate_plugin_id, Capability, PLUGIN_API_VERSION};
use std::collections::BTreeSet;

use crate::{RegistryError, Result};

/// Registry index schema version this client understands.
pub const REGISTRY_SCHEMA_VERSION: u32 = 1;

/// Valid plugin categories (drives the UI category filter).
pub const PLUGIN_CATEGORIES: &[&str] = &["lyrics", "metadata", "visualizer", "widget", "utility"];
/// Valid theme categories.
pub const THEME_CATEGORIES: &[&str] = &["dark", "light", "colorful", "minimal", "retro"];

/// Author identity in the registry (mirrors the manifest author shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryAuthor {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

/// One published version of an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryVersion {
    /// Semver version string.
    pub version: String,
    /// HTTPS URL of the release zip.
    pub download_url: String,
    /// Lowercase hex SHA-256 of the exact zip bytes.
    pub sha256: String,
    /// Advertised zip size in bytes, if known.
    #[serde(default)]
    pub size_bytes: Option<u64>,
    /// ISO-8601 publish timestamp, if known.
    #[serde(default)]
    pub published_at: Option<String>,
    /// Human-readable changelog (required: updates must explain themselves).
    pub changelog: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryCommon {
    id: String,
    name: String,
    description: String,
    author: RegistryAuthor,
    category: String,
    #[serde(default)]
    homepage: Option<String>,
    min_sonora_version: String,
    latest_version: String,
    versions: Vec<RegistryVersion>,
}

/// A plugin listing in `plugins.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPlugin {
    #[serde(flatten)]
    common: RegistryCommon,
    /// Declared capabilities (shown for consent; the installed manifest must
    /// not exceed this set).
    pub capabilities: Vec<String>,
    /// Plugin API version the listed builds target.
    pub api_version: u32,
}

/// A theme listing in `themes.json`. Themes carry no code and no
/// capabilities; only identity + compatibility metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryTheme {
    #[serde(flatten)]
    common: RegistryCommon,
}

macro_rules! impl_common_accessors {
    ($t:ty) => {
        impl $t {
            pub fn id(&self) -> &str {
                &self.common.id
            }
            pub fn name(&self) -> &str {
                &self.common.name
            }
            pub fn description(&self) -> &str {
                &self.common.description
            }
            pub fn author_name(&self) -> &str {
                &self.common.author.name
            }
            pub fn author_url(&self) -> Option<&str> {
                self.common.author.url.as_deref()
            }
            pub fn category(&self) -> &str {
                &self.common.category
            }
            pub fn homepage(&self) -> Option<&str> {
                self.common.homepage.as_deref()
            }
            pub fn min_sonora_version(&self) -> &str {
                &self.common.min_sonora_version
            }
            pub fn latest_version(&self) -> &str {
                &self.common.latest_version
            }
            pub fn versions(&self) -> &[RegistryVersion] {
                &self.common.versions
            }
            /// Look up one published version by exact semver string.
            pub fn version(&self, version: &str) -> Option<&RegistryVersion> {
                self.common.versions.iter().find(|v| v.version == version)
            }
        }
    };
}

impl_common_accessors!(RegistryPlugin);
impl_common_accessors!(RegistryTheme);

impl RegistryPlugin {
    pub fn api_version(&self) -> u32 {
        self.api_version
    }
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }
}

/// A validated registry index (both files merged in memory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub schema: u32,
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub plugins: Vec<RegistryPlugin>,
    #[serde(default)]
    pub themes: Vec<RegistryTheme>,
}

impl RegistryIndex {
    /// Parse and validate a `plugins.json` document.
    pub fn plugins_from_json(text: &str) -> Result<Vec<RegistryPlugin>> {
        let raw: serde_json::Value = serde_json::from_str(text).map_err(|e| {
            RegistryError::InvalidIndex(format!("plugins.json is not valid JSON: {e}"))
        })?;
        let schema = raw
            .get("schema")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        if schema != u64::from(REGISTRY_SCHEMA_VERSION) {
            return Err(RegistryError::InvalidIndex(format!(
                "unsupported registry schema {schema} (client supports v{})",
                REGISTRY_SCHEMA_VERSION
            )));
        }
        let plugins: Vec<RegistryPlugin> = serde_json::from_value(
            raw.get("plugins")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        )
        .map_err(|e| {
            RegistryError::InvalidIndex(format!("plugins.json has invalid entries: {e}"))
        })?;
        let mut ids = BTreeSet::new();
        for p in &plugins {
            validate_plugin_entry(p)?;
            if !ids.insert(p.id().to_string()) {
                return Err(RegistryError::InvalidIndex(format!(
                    "duplicate plugin id '{}'",
                    p.id()
                )));
            }
        }
        Ok(plugins)
    }

    /// Parse and validate a `themes.json` document.
    pub fn themes_from_json(text: &str) -> Result<Vec<RegistryTheme>> {
        let raw: serde_json::Value = serde_json::from_str(text).map_err(|e| {
            RegistryError::InvalidIndex(format!("themes.json is not valid JSON: {e}"))
        })?;
        let schema = raw
            .get("schema")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        if schema != u64::from(REGISTRY_SCHEMA_VERSION) {
            return Err(RegistryError::InvalidIndex(format!(
                "unsupported registry schema {schema} (client supports v{})",
                REGISTRY_SCHEMA_VERSION
            )));
        }
        let themes: Vec<RegistryTheme> = serde_json::from_value(
            raw.get("themes")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        )
        .map_err(|e| {
            RegistryError::InvalidIndex(format!("themes.json has invalid entries: {e}"))
        })?;
        let mut ids = BTreeSet::new();
        for t in &themes {
            validate_theme_entry(t)?;
            if !ids.insert(t.id().to_string()) {
                return Err(RegistryError::InvalidIndex(format!(
                    "duplicate theme id '{}'",
                    t.id()
                )));
            }
        }
        Ok(themes)
    }

    pub fn find_plugin(&self, id: &str) -> Option<&RegistryPlugin> {
        self.plugins.iter().find(|p| p.id() == id)
    }

    pub fn find_theme(&self, id: &str) -> Option<&RegistryTheme> {
        self.themes.iter().find(|t| t.id() == id)
    }
}

fn validate_common(
    common: &RegistryCommon,
    kind: &str,
    categories: &[&str],
) -> Result<Vec<semver::Version>> {
    validate_plugin_id(&common.id)
        .map_err(|e| RegistryError::InvalidIndex(format!("{kind} '{}': {e}", common.id)))?;
    if common.name.trim().is_empty() || common.name.len() > 128 {
        return Err(invalid(kind, &common.id, "name must be 1..=128 characters"));
    }
    if common.description.trim().is_empty() || common.description.len() > 1024 {
        return Err(invalid(
            kind,
            &common.id,
            "description must be 1..=1024 characters",
        ));
    }
    if common.author.name.trim().is_empty() {
        return Err(invalid(kind, &common.id, "author name must not be empty"));
    }
    if !categories.contains(&common.category.as_str()) {
        return Err(invalid(
            kind,
            &common.id,
            &format!(
                "unknown category '{}' (expected one of {})",
                common.category,
                categories.join(", ")
            ),
        ));
    }
    if let Some(home) = &common.homepage {
        require_https(home)
            .map_err(|_| invalid(kind, &common.id, "homepage must be an https:// URL"))?;
    }
    semver::Version::parse(&common.min_sonora_version)
        .map_err(|_| invalid(kind, &common.id, "min_sonora_version must be semver"))?;
    if common.versions.is_empty() {
        return Err(invalid(kind, &common.id, "versions must not be empty"));
    }
    let mut parsed = Vec::with_capacity(common.versions.len());
    let mut seen = BTreeSet::new();
    for v in &common.versions {
        let pv = semver::Version::parse(&v.version).map_err(|_| {
            invalid(
                kind,
                &common.id,
                &format!("version '{}' must be semver", v.version),
            )
        })?;
        if !seen.insert(v.version.clone()) {
            return Err(invalid(
                kind,
                &common.id,
                &format!("duplicate version '{}'", v.version),
            ));
        }
        require_https(&v.download_url)
            .map_err(|_| invalid(kind, &common.id, "download_url must be an https:// URL"))?;
        validate_sha256(&v.sha256).map_err(|_| {
            invalid(
                kind,
                &common.id,
                "sha256 must be 64 lowercase hex characters",
            )
        })?;
        if v.changelog.trim().is_empty() || v.changelog.len() > 4096 {
            return Err(invalid(
                kind,
                &common.id,
                "each version needs a 1..=4096 character changelog",
            ));
        }
        parsed.push(pv);
    }
    if common
        .versions
        .iter()
        .all(|v| v.version != common.latest_version)
    {
        return Err(invalid(
            kind,
            &common.id,
            "latest_version must reference a listed version",
        ));
    }
    Ok(parsed)
}

fn validate_plugin_entry(p: &RegistryPlugin) -> Result<()> {
    validate_common(&p.common, "plugin", PLUGIN_CATEGORIES)?;
    if p.api_version != PLUGIN_API_VERSION {
        return Err(invalid(
            "plugin",
            p.id(),
            &format!(
                "api_version {} not supported by this client (v{})",
                p.api_version, PLUGIN_API_VERSION
            ),
        ));
    }
    if p.capabilities.is_empty() {
        return Err(invalid("plugin", p.id(), "capabilities must not be empty"));
    }
    let mut seen = BTreeSet::new();
    for cap in &p.capabilities {
        cap.parse::<Capability>()
            .map_err(|_| invalid("plugin", p.id(), &format!("unknown capability '{cap}'")))?;
        if !seen.insert(cap.clone()) {
            return Err(invalid(
                "plugin",
                p.id(),
                &format!("duplicate capability '{cap}'"),
            ));
        }
    }
    Ok(())
}

fn validate_theme_entry(t: &RegistryTheme) -> Result<()> {
    validate_common(&t.common, "theme", THEME_CATEGORIES)?;
    Ok(())
}

fn invalid(kind: &str, id: &str, why: &str) -> RegistryError {
    RegistryError::InvalidIndex(format!("{kind} '{id}': {why}"))
}

/// HTTPS-only URL check for registry metadata (no credentials, no spaces).
pub(crate) fn require_https(url: &str) -> Result<()> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| RegistryError::InvalidIndex(format!("URL must use https://: {url}")))?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("").trim();
    if host.is_empty() || host.contains('@') || host.contains(' ') {
        return Err(RegistryError::InvalidIndex(format!(
            "URL has an invalid host: {url}"
        )));
    }
    Ok(())
}

/// Normalize + validate a SHA-256 hex digest.
pub(crate) fn normalize_sha256(hex: &str) -> Result<String> {
    let lower = hex.trim().to_ascii_lowercase();
    if lower.len() != 64 || !lower.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(RegistryError::InvalidIndex(format!(
            "sha256 must be 64 hex characters: {hex}"
        )));
    }
    Ok(lower)
}

fn validate_sha256(hex: &str) -> Result<()> {
    normalize_sha256(hex).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn sample_plugins_json() -> serde_json::Value {
        serde_json::json!({
            "schema": 1,
            "generated_at": "2026-01-01T00:00:00Z",
            "plugins": [{
                "id": "org.sonora.lrclib",
                "name": "LRCLIB Lyrics Provider",
                "description": "Synced lyrics from LRCLIB.",
                "author": {"name": "Sonora Contributors", "url": "https://example.com"},
                "category": "lyrics",
                "homepage": "https://example.com/lrclib",
                "capabilities": ["lyrics:provider", "network:fetch"],
                "api_version": 1,
                "min_sonora_version": "0.1.0",
                "latest_version": "1.0.0",
                "versions": [{
                    "version": "1.0.0",
                    "download_url": "https://example.com/lrclib-1.0.0.zip",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "changelog": "Initial release."
                }]
            }]
        })
    }

    #[test]
    fn accepts_sample_index() {
        let plugins = RegistryIndex::plugins_from_json(&sample_plugins_json().to_string()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].latest_version(), "1.0.0");
    }

    #[test]
    fn rejects_schema_mismatch() {
        let mut v = sample_plugins_json();
        v["schema"] = serde_json::json!(99);
        assert!(RegistryIndex::plugins_from_json(&v.to_string()).is_err());
    }
}
