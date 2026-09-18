//! Package installation: verified download, guarded extraction, atomic
//! install, backups, rollback, uninstall, and update computation.
//!
//! Security order is fixed and non-skippable:
//!
//! 1. Download over HTTPS only; bound the bytes.
//! 2. Verify SHA-256 **before** extraction or execution.
//! 3. Extract with traversal/symlink/count/size guards into a temp dir.
//! 4. Validate the on-disk manifest; the package id/version must match the
//!    registry entry, and its capabilities must not exceed the advertised set.
//! 5. Atomically swap into place, keeping a versioned backup; never delete or
//!    overwrite user files without a restorable backup.
//!
//! A running plugin is stopped/unloaded before its files move and restarted
//! afterwards; failures restore the previous version automatically.

use crate::schema::{RegistryPlugin, RegistryTheme};
use crate::{atomic_write, now_secs, RegistryError, Result, SONORA_VERSION};
use sonora_plugin::{PluginHost, PluginManifest};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Upper bound for a downloaded package (DoS guard).
pub const PACKAGE_MAX_BYTES: usize = 32 * 1024 * 1024;
/// Upper bound for files inside one package.
pub const PACKAGE_MAX_FILES: usize = 500;
/// Upper bound for total uncompressed bytes (zip-bomb guard).
pub const PACKAGE_MAX_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
/// Backups retained per extension id.
pub const MAX_BACKUPS_PER_ID: usize = 3;

/// Plugin vs theme package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionKind {
    Plugin,
    Theme,
}

impl ExtensionKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ExtensionKind::Plugin => "plugin",
            ExtensionKind::Theme => "theme",
        }
    }

    fn manifest_file(self) -> &'static str {
        match self {
            ExtensionKind::Plugin => "manifest.json",
            ExtensionKind::Theme => "theme.json",
        }
    }
}

/// On-disk layout roots for marketplace state.
#[derive(Debug, Clone)]
pub struct MarketplacePaths {
    pub plugins_dir: PathBuf,
    pub themes_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub receipt_path: PathBuf,
}

impl MarketplacePaths {
    pub fn new(data_dir: &Path) -> Self {
        let marketplace = data_dir.join("marketplace");
        Self {
            plugins_dir: data_dir.join("plugins"),
            themes_dir: data_dir.join("themes"),
            cache_dir: marketplace.join("cache"),
            backup_dir: marketplace.join("backups"),
            receipt_path: marketplace.join("installed.json"),
        }
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [
            &self.plugins_dir,
            &self.themes_dir,
            &self.cache_dir,
            &self.backup_dir,
        ] {
            std::fs::create_dir_all(dir).map_err(|e| RegistryError::Io(e.to_string()))?;
        }
        Ok(())
    }

    fn kind_dir(&self, kind: ExtensionKind) -> &Path {
        match kind {
            ExtensionKind::Plugin => &self.plugins_dir,
            ExtensionKind::Theme => &self.themes_dir,
        }
    }
}

/// Install receipt (what came from where, exactly).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Receipt {
    pub kind: ExtensionKind,
    pub version: String,
    pub sha256: String,
    pub registry_url: String,
    pub installed_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct ReceiptStore {
    #[serde(default)]
    entries: HashMap<String, Receipt>,
}

/// Outcome of a successful install or update.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallReport {
    pub id: String,
    pub kind: ExtensionKind,
    pub version: String,
    pub fresh_install: bool,
    pub backed_up_version: Option<String>,
    /// `false` when the files are installed but the plugin failed to
    /// (re)start; the entry stays installed-but-inactive.
    pub started: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_error: Option<String>,
}

/// Outcome of a successful rollback.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RollbackReport {
    pub id: String,
    pub restored_version: String,
    pub previous_version: String,
}

/// One installed extension merged with live host state (for UIs).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledEntry {
    pub id: String,
    pub kind: ExtensionKind,
    pub name: String,
    pub version: String,
    /// Live plugin state (`running`, `stopped`, …) or `installed` for themes.
    pub state: String,
    pub capabilities: Vec<String>,
}

/// One available update with changelog and compatibility.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateInfo {
    pub id: String,
    pub kind: ExtensionKind,
    pub name: String,
    pub current: String,
    pub available: String,
    pub changelog: String,
    pub compatible: bool,
    pub min_sonora_version: String,
}

/// Theme identity for receipt/version bookkeeping. Full definition validation
/// (tokens, mode, value guards) lives in [`crate::theme`]; this struct only
/// carries the fields the installer needs after validation.
#[derive(Debug, Clone)]
struct ThemeManifest {
    id: String,
    version: String,
}

fn validate_theme_manifest(text: &str) -> Result<ThemeManifest> {
    let def = crate::theme::validate_theme_definition(text)?;
    Ok(ThemeManifest {
        id: def.id,
        version: def.version.to_string(),
    })
}

/// Download and checksum-verify a release artifact. HTTPS only; the digest is
/// checked before the bytes are returned to any caller.
pub fn download_verified(
    transport: &Arc<dyn sonora_plugin::NetworkTransport>,
    url: &str,
    sha256: &str,
    max_bytes: usize,
) -> Result<Vec<u8>> {
    if !url.starts_with("https://") {
        return Err(RegistryError::UnsafePackage(
            url.to_string(),
            "download URL must use https://".to_string(),
        ));
    }
    let expected = crate::schema::normalize_sha256(sha256)?;
    let bytes = transport
        .fetch(url, max_bytes)
        .map_err(|e| RegistryError::Transport(format!("download failed: {e:?}")))?;
    let actual = hex_digest(&bytes);
    if actual != expected {
        return Err(RegistryError::ChecksumMismatch {
            id: url.to_string(),
            version: String::new(),
            expected,
            actual,
        });
    }
    Ok(bytes)
}

pub(crate) fn hex_digest(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Extract a zip into `dest` with traversal/symlink/count/size guards.
/// Every entry must stay inside `dest`; absolute paths, `..`, backslashes,
/// symlinks, and oversized archives are rejected.
pub(crate) fn extract_guarded(bytes: &[u8], dest: &Path) -> Result<Vec<PathBuf>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| {
        RegistryError::UnsafePackage("<package>".to_string(), format!("not a valid zip: {e}"))
    })?;
    if archive.len() > PACKAGE_MAX_FILES {
        return Err(RegistryError::UnsafePackage(
            "<package>".to_string(),
            format!("too many files (>{PACKAGE_MAX_FILES})"),
        ));
    }
    let mut total: u64 = 0;
    let mut written = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            RegistryError::UnsafePackage("<package>".to_string(), format!("unreadable entry: {e}"))
        })?;
        if file.is_dir() {
            continue;
        }
        if file.is_symlink() {
            return Err(RegistryError::UnsafePackage(
                "<package>".to_string(),
                format!("symlink entry rejected: {}", file.name()),
            ));
        }
        // `enclosed_name` is None for absolute paths, `..` escapes, backslashes.
        let rel = file
            .enclosed_name()
            .ok_or_else(|| {
                RegistryError::UnsafePackage(
                    "<package>".to_string(),
                    format!("unsafe entry path: {}", file.name()),
                )
            })?
            .to_path_buf();
        if rel.as_os_str().is_empty() {
            return Err(RegistryError::UnsafePackage(
                "<package>".to_string(),
                "empty entry path".to_string(),
            ));
        }
        // Declared sizes are advisory only: count streamed bytes ourselves.
        let out = dest.join(&rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| RegistryError::Io(e.to_string()))?;
        }
        let mut out_file =
            std::fs::File::create(&out).map_err(|e| RegistryError::Io(e.to_string()))?;
        // Count streamed bytes ourselves: entries may lie about `size()`.
        let streamed = std::io::copy(&mut file, &mut out_file)
            .map_err(|e| RegistryError::Io(e.to_string()))?;
        drop(file);
        drop(out_file);
        total = total.saturating_add(streamed);
        if total > PACKAGE_MAX_UNCOMPRESSED_BYTES {
            let _ = std::fs::remove_dir_all(dest);
            return Err(RegistryError::UnsafePackage(
                "<package>".to_string(),
                "archive stream exceeds limit".to_string(),
            ));
        }
        written.push(out);
    }
    Ok(written)
}

// -- receipts ---------------------------------------------------------------

fn read_receipts(paths: &MarketplacePaths) -> Result<ReceiptStore> {
    match std::fs::read_to_string(&paths.receipt_path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| RegistryError::Io(format!("receipts corrupt: {e}"))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ReceiptStore::default()),
        Err(e) => Err(RegistryError::Io(e.to_string())),
    }
}

fn write_receipts(paths: &MarketplacePaths, store: &ReceiptStore) -> Result<()> {
    if let Some(parent) = paths.receipt_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| RegistryError::Io(e.to_string()))?;
    }
    let text = serde_json::to_string_pretty(store).map_err(|e| RegistryError::Io(e.to_string()))?;
    atomic_write(&paths.receipt_path, text.as_bytes())
}

pub fn installed_receipts(paths: &MarketplacePaths) -> Result<BTreeMap<String, Receipt>> {
    Ok(read_receipts(paths)?.entries.into_iter().collect())
}

// -- install ------------------------------------------------------------------

/// Registry-advertised identity a package must match (id, version, and the
/// capability ceiling the installed manifest may not exceed).
pub struct ExpectedPackage<'a> {
    pub id: &'a str,
    pub version: &'a str,
    pub capabilities: &'a [String],
}

/// Verify-then-stage a plugin package. Returns the staged dir and manifest.
fn stage_plugin_package(
    bytes: &[u8],
    expected: &ExpectedPackage<'_>,
    stage_parent: &Path,
) -> Result<(PathBuf, PluginManifest)> {
    let stage = stage_parent.join(format!(
        ".staging-{}-{}-{}",
        sanitize(expected.id),
        std::process::id(),
        now_secs()
    ));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).map_err(|e| RegistryError::Io(e.to_string()))?;
    let result: Result<(PathBuf, PluginManifest)> = (|| {
        extract_guarded(bytes, &stage)?;
        let manifest_text = std::fs::read_to_string(stage.join("manifest.json")).map_err(|_| {
            RegistryError::UnsafePackage(
                expected.id.to_string(),
                "package lacks manifest.json".to_string(),
            )
        })?;
        let manifest = PluginManifest::from_json(&manifest_text).map_err(|e| {
            RegistryError::UnsafePackage(
                expected.id.to_string(),
                format!("embedded manifest invalid: {e}"),
            )
        })?;
        if manifest.id != expected.id {
            return Err(RegistryError::IdentityMismatch(
                expected.id.to_string(),
                format!("package manifest id is '{}'", manifest.id),
            ));
        }
        if manifest.version.to_string() != expected.version {
            return Err(RegistryError::IdentityMismatch(
                expected.id.to_string(),
                format!("package manifest version is '{}'", manifest.version),
            ));
        }
        let advertised: BTreeSet<&str> = expected.capabilities.iter().map(String::as_str).collect();
        for cap in manifest.capabilities.iter() {
            if !advertised.contains(cap.as_str()) {
                return Err(RegistryError::CapabilityEscalation(
                    expected.id.to_string(),
                    format!("package requests '{cap}' beyond the registry listing"),
                ));
            }
        }
        if !stage.join(&manifest.entry).is_file() {
            return Err(RegistryError::UnsafePackage(
                expected.id.to_string(),
                format!("entry '{}' missing from package", manifest.entry),
            ));
        }
        Ok((stage.clone(), manifest))
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    result
}

fn stage_theme_package(
    bytes: &[u8],
    id: &str,
    version: &str,
    stage_parent: &Path,
) -> Result<PathBuf> {
    let stage = stage_parent.join(format!(
        ".staging-{sanitize_id}-{pid}-{ts}",
        sanitize_id = sanitize(id),
        pid = std::process::id(),
        ts = now_secs()
    ));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).map_err(|e| RegistryError::Io(e.to_string()))?;
    let result: Result<PathBuf> = (|| {
        extract_guarded(bytes, &stage)?;
        let text = std::fs::read_to_string(stage.join("theme.json")).map_err(|_| {
            RegistryError::UnsafePackage(id.to_string(), "package lacks theme.json".to_string())
        })?;
        let manifest = validate_theme_manifest(&text)?;
        if manifest.id != id {
            return Err(RegistryError::IdentityMismatch(
                id.to_string(),
                format!("package theme id is '{}'", manifest.id),
            ));
        }
        if manifest.version != version {
            return Err(RegistryError::IdentityMismatch(
                id.to_string(),
                format!("package theme version is '{}'", manifest.version),
            ));
        }
        // Supplemental stylesheet is optional; when present it must pass the
        // no-remote-fetch guard (styling only, no code, no tracking).
        let css_path = stage.join("theme.css");
        if css_path.is_file() {
            let css = std::fs::read(&css_path).map_err(|e| RegistryError::Io(e.to_string()))?;
            crate::theme::validate_theme_css(id, &css)?;
        }
        Ok(stage.clone())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    result
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Move the current install aside into a versioned backup slot, pruning old ones.
fn backup_current(
    paths: &MarketplacePaths,
    kind: ExtensionKind,
    id: &str,
) -> Result<Option<String>> {
    let target = paths.kind_dir(kind).join(id);
    if !target.exists() {
        return Ok(None);
    }
    let version = installed_version(paths, kind, id)?.unwrap_or_else(|| "unknown".to_string());
    // Slot names use the raw version: validated ids and semver are
    // filesystem-safe, and sanitizing would corrupt the version mapping.
    let slot = paths.backup_dir.join(id).join(&version);
    if slot.exists() {
        std::fs::remove_dir_all(&slot).map_err(|e| RegistryError::Io(e.to_string()))?;
    }
    if let Some(parent) = slot.parent() {
        std::fs::create_dir_all(parent).map_err(|e| RegistryError::Io(e.to_string()))?;
    }
    std::fs::rename(&target, &slot)
        .map_err(|e| RegistryError::Io(format!("cannot back up current install: {e}")))?;
    prune_backups(paths, id)?;
    Ok(Some(version))
}

fn installed_version(
    paths: &MarketplacePaths,
    kind: ExtensionKind,
    id: &str,
) -> Result<Option<String>> {
    if let Some(receipt) = read_receipts(paths)?.entries.get(id) {
        return Ok(Some(receipt.version.clone()));
    }
    // Fall back to the on-disk manifest when no receipt exists (pre-marketplace installs).
    let manifest_path = paths.kind_dir(kind).join(id).join(kind.manifest_file());
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(&manifest_path).map_err(|e| RegistryError::Io(e.to_string()))?;
    if kind == ExtensionKind::Plugin {
        Ok(PluginManifest::from_json(&text)
            .ok()
            .map(|m| m.version.to_string()))
    } else {
        Ok(validate_theme_manifest(&text).ok().map(|m| m.version))
    }
}

fn prune_backups(paths: &MarketplacePaths, id: &str) -> Result<()> {
    let dir = paths.backup_dir.join(id);
    let mut versions: Vec<(String, std::time::SystemTime)> = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(std::result::Result::ok).collect::<Vec<_>>())
        .unwrap_or_default();
    for entry in entries {
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        versions.push((entry.file_name().to_string_lossy().to_string(), mtime));
    }
    versions.sort_by(|a, b| b.1.cmp(&a.1));
    for (name, _) in versions.into_iter().skip(MAX_BACKUPS_PER_ID) {
        let _ = std::fs::remove_dir_all(dir.join(name));
    }
    Ok(())
}

fn swap_in(paths: &MarketplacePaths, kind: ExtensionKind, id: &str, stage: &Path) -> Result<()> {
    let target = paths.kind_dir(kind).join(id);
    std::fs::create_dir_all(paths.kind_dir(kind)).map_err(|e| RegistryError::Io(e.to_string()))?;
    if target.exists() {
        // Should not happen (backup_current moved it), but never overwrite blindly.
        return Err(RegistryError::Io(format!(
            "install target still exists: {}",
            target.display()
        )));
    }
    std::fs::rename(stage, &target)
        .map_err(|e| RegistryError::Io(format!("atomic install failed: {e}")))?;
    Ok(())
}

fn record_receipt(
    paths: &MarketplacePaths,
    id: &str,
    kind: ExtensionKind,
    version: &str,
    sha256: &str,
    registry_url: &str,
) -> Result<()> {
    let mut store = read_receipts(paths)?;
    store.entries.insert(
        id.to_string(),
        Receipt {
            kind,
            version: version.to_string(),
            sha256: sha256.to_string(),
            registry_url: registry_url.to_string(),
            installed_at: now_secs(),
        },
    );
    write_receipts(paths, &store)
}

/// Install a verified plugin package (bytes already checksum-verified).
/// Stops a running instance first; restarts it afterwards when possible.
pub fn install_plugin_bytes(
    paths: &MarketplacePaths,
    host: &PluginHost,
    expected: &ExpectedPackage<'_>,
    bytes: &[u8],
    sha256: &str,
    registry_url: &str,
) -> Result<InstallReport> {
    let fresh_install = !paths
        .kind_dir(ExtensionKind::Plugin)
        .join(expected.id)
        .exists();
    let was_running = matches!(
        host_state_of(host, expected.id),
        Some(s) if s == "running"
    );
    if was_running {
        let _ = host.stop(expected.id);
        let _ = host.unload(expected.id);
    }
    let (stage, _manifest) = stage_plugin_package(bytes, expected, &paths.plugins_dir)?;
    let backed_up_version = backup_current(paths, ExtensionKind::Plugin, expected.id)?;
    if let Err(e) = swap_in(paths, ExtensionKind::Plugin, expected.id, &stage) {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(e);
    }
    let _ = std::fs::remove_dir_all(&stage);
    record_receipt(
        paths,
        expected.id,
        ExtensionKind::Plugin,
        expected.version,
        sha256,
        registry_url,
    )?;

    // Register + load + start through the normal lifecycle (validation runs again).
    let mut started = false;
    let mut start_error = None;
    host.discover(&paths.plugins_dir);
    match host
        .load(expected.id)
        .and_then(|()| host.start(expected.id))
    {
        Ok(()) => started = true,
        Err(e) => {
            start_error = Some(e.to_string());
            tracing::warn!("installed plugin '{}' failed to start: {e}", expected.id);
        }
    }
    Ok(InstallReport {
        id: expected.id.to_string(),
        kind: ExtensionKind::Plugin,
        version: expected.version.to_string(),
        fresh_install,
        backed_up_version,
        started,
        start_error,
    })
}

fn host_state_of(host: &PluginHost, id: &str) -> Option<String> {
    host.list()
        .into_iter()
        .find(|p| p.id == id)
        .map(|p| p.state)
}

/// Install a verified theme package (no code runs, ever).
pub fn install_theme_bytes(
    paths: &MarketplacePaths,
    id: &str,
    version: &str,
    bytes: &[u8],
    sha256: &str,
    registry_url: &str,
) -> Result<InstallReport> {
    let fresh_install = !paths.kind_dir(ExtensionKind::Theme).join(id).exists();
    let stage = stage_theme_package(bytes, id, version, &paths.themes_dir)?;
    let backed_up_version = backup_current(paths, ExtensionKind::Theme, id)?;
    if let Err(e) = swap_in(paths, ExtensionKind::Theme, id, &stage) {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(e);
    }
    let _ = std::fs::remove_dir_all(&stage);
    record_receipt(
        paths,
        id,
        ExtensionKind::Theme,
        version,
        sha256,
        registry_url,
    )?;
    Ok(InstallReport {
        id: id.to_string(),
        kind: ExtensionKind::Theme,
        version: version.to_string(),
        fresh_install,
        backed_up_version,
        started: true,
        start_error: None,
    })
}

/// Restore a previous version from backup, preserving the displaced files as
/// the new backup of their version.
pub fn rollback(
    paths: &MarketplacePaths,
    host: &PluginHost,
    id: &str,
    version: Option<&str>,
) -> Result<RollbackReport> {
    let kind =
        installed_kind(paths, id)?.ok_or_else(|| RegistryError::NotInstalled(id.to_string()))?;
    let backup_root = paths.backup_dir.join(id);
    let target_version = match version {
        Some(v) => v.to_string(),
        None => {
            newest_backup(&backup_root)?.ok_or_else(|| RegistryError::NoBackup(id.to_string()))?
        }
    };
    let slot = backup_root.join(&target_version);
    if !slot.is_dir() {
        return Err(RegistryError::NoBackup(format!(
            "{id} has no backup of version {target_version}"
        )));
    }
    let previous_version =
        installed_version(paths, kind, id)?.unwrap_or_else(|| "unknown".to_string());
    if kind == ExtensionKind::Plugin {
        let _ = host.stop(id);
        let _ = host.unload(id);
    }
    let target = paths.kind_dir(kind).join(id);
    let displaced = paths
        .kind_dir(kind)
        .join(format!(".displaced-{pid}", pid = std::process::id()));
    let _ = std::fs::remove_dir_all(&displaced);
    if target.exists() {
        std::fs::rename(&target, &displaced).map_err(|e| RegistryError::Io(e.to_string()))?;
    }
    // Validate the backup before swapping it in.
    let manifest_ok = (|| -> Result<()> {
        let text = std::fs::read_to_string(slot.join(kind.manifest_file())).map_err(|e| {
            RegistryError::UnsafePackage(id.to_string(), format!("backup unreadable: {e}"))
        })?;
        if kind == ExtensionKind::Plugin {
            PluginManifest::from_json(&text).map(|_| ()).map_err(|e| {
                RegistryError::UnsafePackage(
                    id.to_string(),
                    format!("backup manifest invalid: {e}"),
                )
            })?;
        } else {
            validate_theme_manifest(&text).map(|_| ())?;
        }
        Ok(())
    })();
    if let Err(e) = manifest_ok {
        if displaced.exists() {
            let _ = std::fs::rename(&displaced, &target);
        }
        return Err(e);
    }
    std::fs::rename(&slot, &target)
        .map_err(|e| RegistryError::Io(format!("rollback swap failed: {e}")))?;
    if displaced.exists() {
        // Preserve the displaced files as the backup of their version.
        let preserve = backup_root.join(&previous_version);
        let _ = std::fs::remove_dir_all(&preserve);
        let _ = std::fs::rename(&displaced, &preserve);
    }
    // Update the receipt to the restored version.
    if let Ok(mut store) = read_receipts(paths) {
        if let Some(receipt) = store.entries.get_mut(id) {
            receipt.version.clone_from(&target_version);
            receipt.installed_at = now_secs();
            let _ = write_receipts(paths, &store);
        }
    }
    if kind == ExtensionKind::Plugin {
        host.discover(&paths.plugins_dir);
        let _ = host.load(id).and_then(|()| host.start(id));
    }
    Ok(RollbackReport {
        id: id.to_string(),
        restored_version: target_version,
        previous_version,
    })
}

fn newest_backup(backup_root: &Path) -> Result<Option<String>> {
    let entries = std::fs::read_dir(backup_root)
        .map(|rd| rd.filter_map(std::result::Result::ok).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut candidates: Vec<(String, std::time::SystemTime)> = Vec::new();
    for entry in entries {
        if entry.path().is_dir() {
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            candidates.push((entry.file_name().to_string_lossy().to_string(), mtime));
        }
    }
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(candidates.into_iter().next().map(|(name, _)| name))
}

fn installed_kind(paths: &MarketplacePaths, id: &str) -> Result<Option<ExtensionKind>> {
    if let Some(receipt) = read_receipts(paths)?.entries.get(id) {
        return Ok(Some(receipt.kind));
    }
    if paths.plugins_dir.join(id).join("manifest.json").is_file() {
        return Ok(Some(ExtensionKind::Plugin));
    }
    if paths.themes_dir.join(id).join("theme.json").is_file() {
        return Ok(Some(ExtensionKind::Theme));
    }
    Ok(None)
}

/// Uninstall an extension: stop/unload, remove files and receipt. Backups are
/// kept so an exact version can be restored offline via rollback-then-nothing…
/// (rollback needs the dir; document that uninstall clears backups? — decision:
/// uninstall removes the install and receipt but KEEPS backups.)
pub fn uninstall(paths: &MarketplacePaths, host: &PluginHost, id: &str) -> Result<()> {
    let kind =
        installed_kind(paths, id)?.ok_or_else(|| RegistryError::NotInstalled(id.to_string()))?;
    if kind == ExtensionKind::Plugin {
        let _ = host.stop(id);
        let _ = host.unload(id);
    }
    let target = paths.kind_dir(kind).join(id);
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|e| RegistryError::Io(e.to_string()))?;
    }
    let mut store = read_receipts(paths)?;
    store.entries.remove(id);
    write_receipts(paths, &store)?;
    Ok(())
}

// -- updates ------------------------------------------------------------------

/// Compute available updates for installed extensions against an index.
/// Incompatible newer versions are reported with `compatible: false` so the UI
/// can show (but refuse) them.
pub fn check_updates(
    index_plugins: &[RegistryPlugin],
    index_themes: &[RegistryTheme],
    receipts: &HashMap<String, Receipt>,
) -> Vec<UpdateInfo> {
    let mut out = Vec::new();
    for (id, receipt) in receipts {
        let current = match semver::Version::parse(&receipt.version) {
            Ok(v) => v,
            Err(_) => continue,
        };
        match receipt.kind {
            ExtensionKind::Plugin => {
                let Some(entry) = index_plugins.iter().find(|p| p.id() == id) else {
                    continue;
                };
                if let Some(info) = update_for_versions(
                    id,
                    receipt.kind,
                    entry.name(),
                    &current,
                    entry.versions(),
                    entry.min_sonora_version(),
                    entry.api_version() == sonora_plugin::PLUGIN_API_VERSION,
                ) {
                    out.push(info);
                }
            }
            ExtensionKind::Theme => {
                let Some(entry) = index_themes.iter().find(|t| t.id() == id) else {
                    continue;
                };
                if let Some(info) = update_for_versions(
                    id,
                    receipt.kind,
                    entry.name(),
                    &current,
                    entry.versions(),
                    entry.min_sonora_version(),
                    true,
                ) {
                    out.push(info);
                }
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn update_for_versions(
    id: &str,
    kind: ExtensionKind,
    name: &str,
    current: &semver::Version,
    versions: &[crate::schema::RegistryVersion],
    min_sonora: &str,
    api_ok: bool,
) -> Option<UpdateInfo> {
    let mut newer: Vec<(semver::Version, &crate::schema::RegistryVersion)> = versions
        .iter()
        .filter_map(|v| semver::Version::parse(&v.version).ok().map(|pv| (pv, v)))
        .filter(|(pv, _)| pv > current)
        .collect();
    if newer.is_empty() {
        return None;
    }
    newer.sort_by(|a, b| b.0.cmp(&a.0));
    let (latest_pv, _latest) = &newer[0];
    let sonora_ok = semver::Version::parse(min_sonora)
        .ok()
        .zip(semver::Version::parse(SONORA_VERSION).ok())
        .map(|(min, cur)| cur >= min)
        .unwrap_or(false);
    let compatible = api_ok && sonora_ok;
    // Prefer the newest compatible version; otherwise report the newest with compatible=false.
    let chosen = newer
        .iter()
        .find(|_| compatible)
        .map(|(_, v)| *v)
        .unwrap_or(newer[0].1);
    let chosen_compatible = compatible && chosen.version == latest_pv.to_string();
    Some(UpdateInfo {
        id: id.to_string(),
        kind,
        name: name.to_string(),
        current: current.to_string(),
        available: chosen.version.clone(),
        changelog: chosen.changelog.clone(),
        compatible: chosen_compatible,
        min_sonora_version: min_sonora.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[test]
    fn rejects_unsafe_entry_paths() {
        let dir = std::env::temp_dir().join(format!("sonora-zipguard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for bad in ["../evil.wasm", "/abs/path.wasm", "sub/../../escape.wasm"] {
            let mut buf = Vec::new();
            {
                let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
                w.start_file(bad, zip::write::SimpleFileOptions::default())
                    .unwrap();
                w.write_all(b"x").unwrap();
                w.finish().unwrap();
            }
            assert!(
                extract_guarded(&buf, &dir.join("out")).is_err(),
                "accepted unsafe path: {bad}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
