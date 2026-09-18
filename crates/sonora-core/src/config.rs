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
        let data_dir = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut home = std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."));
                home.push(".local");
                home.push("share");
                home
            })
            .join("sonora");

        Self {
            data_dir,
            music_dirs: Vec::new(),
            default_volume: 1.0,
        }
    }
}
