//! Closed capability model for Sonora plugins.
//!
//! A plugin may only use the host APIs listed in its manifest. Enforcement is
//! three-layered:
//!
//! 1. **Manifest validation** — unknown capabilities and inconsistent
//!    declarations (e.g. `network:fetch` without `allowed_domains`) are
//!    rejected before anything loads.
//! 2. **Load-time import check** — a WASM module may only import the host
//!    functions its capabilities entitle it to; anything else (including WASI)
//!    fails the load.
//! 3. **Call-time gating** — every host function re-checks the capability, and
//!    every plugin export is callable only when the matching capability was
//!    declared.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The complete, closed set of v1 capabilities.
///
/// Serialized form uses the `domain:action` strings from the spec
/// (`"library:read"`, `"network:fetch"`, …). Unknown strings fail to parse,
/// which keeps the surface small and stable by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Read-only library statistics snapshot (`sonora.lib_stats`).
    #[serde(rename = "library:read")]
    LibraryRead,
    /// Answer metadata queries (`metadata_fetch` export).
    #[serde(rename = "metadata:read")]
    MetadataRead,
    /// Answer lyrics queries (`lyrics_fetch` export + registration in the
    /// lyrics cascade).
    #[serde(rename = "lyrics:provider")]
    LyricsProvider,
    /// Read the latest visualizer FFT frame (`sonora.tap_read`).
    #[serde(rename = "visualizer:tap")]
    VisualizerTap,
    /// Describe a UI widget slot (`widget_describe` export).
    #[serde(rename = "ui:widget")]
    UiWidget,
    /// Per-plugin isolated key/value cache (`sonora.cache_get/put`).
    #[serde(rename = "storage:cache")]
    StorageCache,
    /// Allow-listed HTTPS fetch stub (`sonora.net_fetch`).
    #[serde(rename = "network:fetch")]
    NetworkFetch,
}

impl Capability {
    /// All capabilities known to API v1.
    pub const ALL: [Capability; 7] = [
        Capability::LibraryRead,
        Capability::MetadataRead,
        Capability::LyricsProvider,
        Capability::VisualizerTap,
        Capability::UiWidget,
        Capability::StorageCache,
        Capability::NetworkFetch,
    ];

    /// Canonical string form used in manifests and error messages.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Capability::LibraryRead => "library:read",
            Capability::MetadataRead => "metadata:read",
            Capability::LyricsProvider => "lyrics:provider",
            Capability::VisualizerTap => "visualizer:tap",
            Capability::UiWidget => "ui:widget",
            Capability::StorageCache => "storage:cache",
            Capability::NetworkFetch => "network:fetch",
        }
    }

    /// Parse a canonical capability string. Unknown strings are rejected.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Capability {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("unknown capability '{s}'"))
    }
}

/// A validated, duplicate-free capability set from a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    inner: BTreeSet<Capability>,
}

impl CapabilitySet {
    pub fn contains(&self, cap: Capability) -> bool {
        self.inner.contains(&cap)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = Capability> + '_ {
        self.inner.iter().copied()
    }

    /// Host function imports this set entitles a module to.
    /// `sonora.log` is always available and needs no capability.
    pub fn allowed_imports(&self) -> BTreeSet<&'static str> {
        let mut out = BTreeSet::from(["log"]);
        if self.contains(Capability::StorageCache) {
            out.insert("cache_get");
            out.insert("cache_put");
        }
        if self.contains(Capability::NetworkFetch) {
            out.insert("net_fetch");
            out.insert("net_last_status");
        }
        if self.contains(Capability::LibraryRead) {
            out.insert("lib_stats");
        }
        if self.contains(Capability::VisualizerTap) {
            out.insert("tap_read");
        }
        out
    }

    /// Plugin exports that require a declared capability. Returns the missing
    /// capability for the first undeclared export found, if any.
    pub fn missing_capability_for_exports<'a>(
        &self,
        exports: impl Iterator<Item = &'a str>,
    ) -> Option<Capability> {
        for export in exports {
            let required = match export {
                "lyrics_fetch" | "lyrics_result_ptr" | "lyrics_result_len" => {
                    Some(Capability::LyricsProvider)
                }
                "metadata_fetch" | "metadata_result_ptr" | "metadata_result_len" => {
                    Some(Capability::MetadataRead)
                }
                "visualizer_info" | "visualizer_result_ptr" | "visualizer_result_len" => {
                    Some(Capability::VisualizerTap)
                }
                "widget_describe" | "widget_result_ptr" | "widget_result_len" => {
                    Some(Capability::UiWidget)
                }
                _ => None,
            };
            if let Some(cap) = required {
                if !self.contains(cap) {
                    return Some(cap);
                }
            }
        }
        None
    }
}

impl From<BTreeSet<Capability>> for CapabilitySet {
    fn from(inner: BTreeSet<Capability>) -> Self {
        Self { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_canonical_strings() {
        for cap in Capability::ALL {
            assert_eq!(Capability::parse(cap.as_str()), Some(cap));
        }
    }

    #[test]
    fn rejects_unknown_capability() {
        assert_eq!(Capability::parse("filesystem:write"), None);
        assert_eq!(Capability::parse("audio:dsp"), None);
        assert_eq!(Capability::parse(""), None);
    }
}
