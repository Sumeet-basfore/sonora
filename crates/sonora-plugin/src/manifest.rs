//! `manifest.json` v1: parsing and fail-closed validation.
//!
//! Validation always runs **before** any plugin code loads. Any violation —
//! malformed JSON, bad id/version, unknown capability, `network:fetch`
//! without an HTTPS allow-list, unsafe `entry` paths — rejects the plugin.

use crate::capability::{Capability, CapabilitySet};
use crate::{PluginError, Result, PLUGIN_API_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

/// Expected manifest file name inside a plugin directory.
pub const PLUGIN_MANIFEST_FILENAME: &str = "manifest.json";

/// Author identity. Accepts either a plain string (`"author": "Name"`) or the
/// `{ "name", "url" }` object from the plugin-system spec for compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginAuthor {
    Name(String),
    Detailed { name: String, url: Option<String> },
}

impl PluginAuthor {
    pub fn name(&self) -> &str {
        match self {
            PluginAuthor::Name(n) => n,
            PluginAuthor::Detailed { name, .. } => name,
        }
    }
}

/// Raw manifest as deserialized from `manifest.json`. Use [`PluginManifest`]
/// (validated) everywhere else.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: PluginAuthor,
    #[serde(default)]
    pub description: String,
    pub api_version: u32,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub entry: String,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
}

/// Validated plugin manifest v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: semver::Version,
    pub author: PluginAuthor,
    pub description: String,
    pub api_version: u32,
    pub capabilities: CapabilitySet,
    /// WASM file name inside the plugin directory (single file name only).
    pub entry: String,
    /// HTTPS origins the plugin may contact via `sonora.net_fetch`.
    pub allowed_domains: Vec<String>,
}

impl PluginManifest {
    /// Parse and validate a manifest from JSON text.
    pub fn from_json(text: &str) -> Result<Self> {
        let raw: RawManifest = serde_json::from_str(text)
            .map_err(|e| PluginError::InvalidManifest(format!("malformed JSON: {e}")))?;
        Self::validate(raw)
    }

    /// Parse and validate a manifest from a file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| PluginError::Io(format!("cannot read {}: {e}", path.display())))?;
        Self::from_json(&text)
    }

    fn validate(raw: RawManifest) -> Result<Self> {
        validate_plugin_id(&raw.id)?;
        validate_name(&raw.name)?;
        let version = validate_version(&raw.version)?;
        validate_author(&raw.author)?;
        validate_description(&raw.description)?;

        if raw.api_version != PLUGIN_API_VERSION {
            return Err(PluginError::InvalidManifest(format!(
                "unsupported api_version {} (host supports v{})",
                raw.api_version, PLUGIN_API_VERSION
            )));
        }

        let capabilities = validate_capabilities(&raw.capabilities)?;
        let entry = validate_entry(&raw.entry)?;
        let allowed_domains = validate_allowed_domains(&raw.allowed_domains)?;

        let wants_net = capabilities.contains(Capability::NetworkFetch);
        if wants_net && allowed_domains.is_empty() {
            return Err(PluginError::InvalidManifest(
                "capability 'network:fetch' requires a non-empty 'allowed_domains' allow-list"
                    .to_string(),
            ));
        }
        if !wants_net && !allowed_domains.is_empty() {
            return Err(PluginError::InvalidManifest(
                "'allowed_domains' is declared but capability 'network:fetch' is not requested"
                    .to_string(),
            ));
        }

        Ok(Self {
            id: raw.id,
            name: raw.name,
            version,
            author: raw.author,
            description: raw.description,
            api_version: raw.api_version,
            capabilities,
            entry,
            allowed_domains,
        })
    }

    /// Returns `true` when `url` matches the manifest's HTTPS allow-list
    /// (exact origin or sub-domain of an entry).
    #[must_use]
    pub fn is_url_allowed(&self, url: &str) -> bool {
        if !self.capabilities.contains(Capability::NetworkFetch) {
            return false;
        }
        let host = match url_host(url) {
            Some(h) => h,
            None => return false,
        };
        self.allowed_domains.iter().any(|entry| {
            let allowed = entry
                .strip_prefix("https://")
                .unwrap_or(entry)
                .trim_end_matches('/')
                .to_ascii_lowercase();
            host == allowed || host.ends_with(&format!(".{allowed}"))
        })
    }
}

/// Reverse-DNS style id: ≥2 dot-separated labels, lowercase alnum/underscore,
/// each label starting with an ASCII letter or digit.
///
/// Public so the registry index (untrusted metadata) is held to the same
/// identifier rules as installed manifests.
pub fn validate_plugin_id(id: &str) -> Result<()> {
    let invalid = |why: &str| PluginError::InvalidManifest(format!("invalid id '{id}': {why}"));
    let labels: Vec<&str> = id.split('.').collect();
    if labels.len() < 2 {
        return Err(invalid("must be reverse-DNS with at least two labels"));
    }
    for label in labels {
        if label.is_empty() || label.len() > 63 {
            return Err(invalid("empty or overlong label"));
        }
        let mut chars = label.chars();
        let first = chars.next().unwrap_or_default();
        if !first.is_ascii_alphanumeric() {
            return Err(invalid("each label must start with a letter or digit"));
        }
        if !label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(invalid("labels may only contain a-z, 0-9 and underscore"));
        }
    }
    if id.len() > 128 {
        return Err(invalid("must be at most 128 characters"));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.len() > 128 {
        return Err(PluginError::InvalidManifest(
            "name must be 1..=128 characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_version(version: &str) -> Result<semver::Version> {
    semver::Version::parse(version)
        .map_err(|_| PluginError::InvalidManifest(format!("invalid semver version '{version}'")))
}

fn validate_author(author: &PluginAuthor) -> Result<()> {
    if author.name().trim().is_empty() {
        return Err(PluginError::InvalidManifest(
            "author name must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_description(description: &str) -> Result<()> {
    if description.trim().is_empty() || description.len() > 1024 {
        return Err(PluginError::InvalidManifest(
            "description must be 1..=1024 characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_capabilities(caps: &[String]) -> Result<CapabilitySet> {
    if caps.is_empty() {
        return Err(PluginError::InvalidManifest(
            "at least one capability must be requested".to_string(),
        ));
    }
    let mut set = BTreeSet::new();
    for raw in caps {
        let cap: Capability = raw
            .parse()
            .map_err(|_| PluginError::InvalidManifest(format!("unknown capability '{raw}'")))?;
        if !set.insert(cap) {
            return Err(PluginError::InvalidManifest(format!(
                "duplicate capability '{raw}'"
            )));
        }
    }
    Ok(CapabilitySet::from(set))
}

/// The entry must be a single `.wasm` file name — no directories, no
/// absolute paths, no traversal — resolved inside the plugin directory.
fn validate_entry(entry: &str) -> Result<String> {
    if entry.is_empty() || !entry.ends_with(".wasm") {
        return Err(PluginError::InvalidManifest(
            "entry must be a '.wasm' file name".to_string(),
        ));
    }
    if entry.contains('/') || entry.contains('\\') || entry.contains("..") {
        return Err(PluginError::InvalidManifest(
            "entry must be a plain file name inside the plugin directory".to_string(),
        ));
    }
    Ok(entry.to_string())
}

fn validate_allowed_domains(domains: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::with_capacity(domains.len());
    for d in domains {
        let lower = d.to_ascii_lowercase();
        let origin = lower.strip_prefix("https://").ok_or_else(|| {
            PluginError::InvalidManifest(format!("allowed_domains entry '{d}' must use https://"))
        })?;
        let host = origin.trim_end_matches('/');
        if host.is_empty() || host.contains('/') || host.contains(' ') || host.contains('@') {
            return Err(PluginError::InvalidManifest(format!(
                "allowed_domains entry '{d}' must be a bare https:// origin"
            )));
        }
        out.push(format!("https://{host}"));
    }
    Ok(out)
}

/// Extract the lowercase host from an `https://` URL. Returns `None` for
/// anything else (non-HTTPS, malformed, raw IPs are rejected at the
/// allow-list layer by construction of validated entries).
fn url_host(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let host = rest
        .split(['/', '?', '#'])
        .next()?
        .trim()
        .to_ascii_lowercase();
    if host.is_empty() || host.contains('@') || host.contains(' ') {
        return None;
    }
    Some(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_json() -> serde_json::Value {
        serde_json::json!({
            "id": "org.example.lyrics",
            "name": "Example Lyrics",
            "version": "1.0.0",
            "author": "Example Author",
            "description": "Minimal example lyrics provider.",
            "api_version": 1,
            "capabilities": ["lyrics:provider", "storage:cache"],
            "entry": "plugin.wasm",
            "allowed_domains": []
        })
    }

    #[test]
    fn accepts_valid_manifest() {
        let m = PluginManifest::from_json(&base_json().to_string()).unwrap();
        assert_eq!(m.id, "org.example.lyrics");
        assert!(m.capabilities.contains(Capability::LyricsProvider));
    }

    #[test]
    fn rejects_unknown_capability() {
        let mut v = base_json();
        v["capabilities"] = serde_json::json!(["lyrics:provider", "filesystem:write"]);
        assert!(PluginManifest::from_json(&v.to_string()).is_err());
    }
}
