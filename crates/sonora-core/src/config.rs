use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Sonora runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonoraConfig {
    pub data_dir: PathBuf,
    pub music_dirs: Vec<PathBuf>,
    pub default_volume: f32,
}

impl Default for SonoraConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            music_dirs: Vec::new(),
            default_volume: 1.0,
        }
    }
}

/// Platform-appropriate data directory:
///
/// - Windows: `%LOCALAPPDATA%\Sonora` (fall back to `%USERPROFILE%`)
/// - macOS: `~/Library/Application Support/Sonora`
/// - Linux/other: `$XDG_DATA_HOME/sonora`, else `~/.local/share/sonora`
/// - Final fallback: a `./sonora-data` directory, to avoid writing dotfiles
///   into an unrelated working directory.
fn default_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            return local.join("Sonora");
        }
        if let Some(profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
            return profile.join("Sonora");
        }
        return PathBuf::from(".").join("sonora-data");
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return home
                .join("Library")
                .join("Application Support")
                .join("Sonora");
        }
        return PathBuf::from(".").join("sonora-data");
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
            return xdg.join("sonora");
        }
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return home.join(".local").join("share").join("sonora");
        }
        PathBuf::from(".").join("sonora-data")
    }
}
