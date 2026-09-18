//! Git-backed extension registry foundation for Sonora.
//!
//! A registry is a static file tree (mirrored from git to any static host):
//! `v1/plugins.json` + `v1/themes.json` plus out-of-band release zips. This
//! crate implements the client side:
//!
//! - [`schema`] — registry index format (`plugins.json` / `themes.json`) and
//!   fail-closed validation. Registry metadata is **untrusted input**.
//! - [`client`] — index fetch, schema validation, atomic local cache, and
//!   graceful offline fallback to the last good cache.
//! - [`installer`] — checksum-verified download, traversal-guarded atomic
//!   install, backups, rollback, uninstall, and update computation.
//! - [`marketplace`] — the service tying registry + installer + [`PluginHost`]
//!   together (also the UI backend).
//!
//! Out of scope: payments, accounts, ratings/reviews, social features, and
//! decentralized package signing (checksums only in v1).

pub mod client;
pub mod installer;
pub mod marketplace;
pub mod schema;
pub mod theme;

pub use client::{RegistryClient, ResolvedIndex, DEFAULT_REGISTRY_URL, REGISTRY_CACHE_TTL_SECS};
pub use installer::{
    ExpectedPackage, ExtensionKind, InstallReport, InstalledEntry, MarketplacePaths, Receipt,
    RollbackReport, UpdateInfo, MAX_BACKUPS_PER_ID,
};
pub use marketplace::{Catalog, CatalogEntry, InstalledTheme, Marketplace};
pub use schema::{
    RegistryAuthor, RegistryIndex, RegistryPlugin, RegistryTheme, RegistryVersion,
    PLUGIN_CATEGORIES, REGISTRY_SCHEMA_VERSION, THEME_CATEGORIES,
};
pub use theme::{ThemeDefinition, ThemeMode, REQUIRED_THEME_TOKENS};

/// Sonora release version this client enforces `min_sonora_version` against.
pub const SONORA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Error type for registry operations.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("invalid registry index: {0}")]
    InvalidIndex(String),
    #[error("registry transport error: {0}")]
    Transport(String),
    #[error("registry unavailable offline and no cached index exists")]
    Offline,
    #[error("checksum mismatch for {id} {version}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        id: String,
        version: String,
        expected: String,
        actual: String,
    },
    #[error("unsafe package for {0}: {1}")]
    UnsafePackage(String, String),
    #[error("package {0} requests capabilities beyond the registry listing: {1}")]
    CapabilityEscalation(String, String),
    #[error("package {0} identity mismatch: {1}")]
    IdentityMismatch(String, String),
    #[error("no compatible version of {0} ({1})")]
    NoCompatibleVersion(String, String),
    #[error("{0} has no update available")]
    NoUpdateAvailable(String),
    #[error("{0} is already installed at requested version")]
    AlreadyInstalled(String),
    #[error("no backup available for {0}")]
    NoBackup(String),
    #[error("{0} is not installed")]
    NotInstalled(String),
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<RegistryError> for sonora_common::SonoraError {
    fn from(e: RegistryError) -> Self {
        Self::Marketplace(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, RegistryError>;

/// Current Unix timestamp (seconds). Used for cache metadata and receipts.
pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Atomically write `bytes` to `path` (tmp file + rename in the same dir).
pub(crate) fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        now_secs() % 1_000_000
    ));
    std::fs::write(&tmp, bytes).map_err(|e| RegistryError::Io(e.to_string()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        RegistryError::Io(e.to_string())
    })?;
    Ok(())
}
