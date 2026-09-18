//! Control-plane commands.
//!
//! [`SonoraCommand`] is the mutation vocabulary of the app: every state
//! change flows through [`SonoraApp::handle_command`](crate::SonoraApp::handle_command),
//! which routes to the owning service. Read-only queries (`search`, `status`,
//! …) stay as direct method calls — commands are for mutations only.

use sonora_common::TrackId;
use sonora_plugin::PluginManifest;
use std::path::PathBuf;

/// Mutations the application can perform.
#[derive(Debug)]
pub enum SonoraCommand {
    PlayFile(PathBuf),
    PlayTrack(TrackId),
    PlayAlbum(i64),
    PlayQueueIndex(usize),
    Pause,
    Resume,
    Stop,
    Seek(u64),
    SetVolume(f32),
    QueueNext,
    QueuePrevious,
    EnqueueTrack(TrackId),
    RemoveFromQueue(usize),
    MoveQueueItem {
        from: usize,
        to: usize,
    },
    ClearQueue,
    ScanDirectory(PathBuf),
    PluginRegister {
        manifest: Box<PluginManifest>,
        dir: PathBuf,
    },
    PluginLoad {
        id: String,
    },
    PluginLoadBytes {
        id: String,
        wasm: Vec<u8>,
    },
    PluginStart {
        id: String,
    },
    PluginStop {
        id: String,
    },
    PluginUnload {
        id: String,
    },
    // -- marketplace mutations (reads stay as direct `market_*` methods) --
    RegistryRefresh,
    MarketInstall {
        id: String,
        version: Option<String>,
    },
    MarketUpdate {
        id: String,
    },
    MarketRollback {
        id: String,
        version: Option<String>,
    },
    MarketUninstall {
        id: String,
    },
    MarketSetActiveTheme {
        id: Option<String>,
    },
}
