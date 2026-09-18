//! Subsystem services.
//!
//! Each service owns exactly one subsystem (library DB, audio player, queue,
//! lyrics pipeline) plus a handle to the [`EventBus`](crate::bus::EventBus).
//! Services never reach into each other: cross-cutting flows (e.g. "play a
//! library track") are orchestrated by [`SonoraApp`](crate::SonoraApp), which
//! only uses the services' public APIs and the event stream.

pub mod library;
pub mod lyrics;
pub mod playback;
pub mod queue;

pub use library::LibraryService;
pub use lyrics::LyricsService;
pub use playback::PlaybackService;
pub use queue::QueueService;
