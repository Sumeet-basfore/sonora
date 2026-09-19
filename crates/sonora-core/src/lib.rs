pub mod app;
pub mod artwork;
pub mod bus;
pub mod cache;
pub mod command;
pub mod config;
pub mod enrichment;
pub mod event;
pub mod lyrics_online;
pub mod metadata;
pub mod queue;
pub mod services;

pub use app::{PlaybackStatus, SonoraApp};
pub use artwork::{
    ArtworkCandidate, ArtworkKind, ArtworkPipeline, ArtworkProvider, ArtworkQuery,
    ArtworkSourceType, CachedArtworkAsset, CoverArtArchiveClient, FanartTvArtworkProvider,
    WikidataArtworkProvider, FULL_ARTWORK_DIMENSION, MAX_IMAGE_DIMENSION, MAX_IMAGE_PAYLOAD_BYTES,
    THUMBNAIL_DIMENSION,
};
pub use bus::EventBus;
pub use cache::{
    OnlineCacheManager, DEFAULT_ARTWORK_TTL_SECS, DEFAULT_LYRICS_TTL_SECS,
    DEFAULT_METADATA_TTL_SECS,
};
pub use command::SonoraCommand;
pub use config::SonoraConfig;
pub use enrichment::EnrichmentRouter;
pub use event::SonoraEvent;
pub use lyrics_online::{
    LrclibClient, LyricsCandidate, LyricsCandidateQuery, LyricsSourceKind, LyricsSyncType,
    OnlineLyricsProvider,
};
pub use metadata::{
    ConfidenceTier, DeterministicMatcher, LocalTrackMetadata, MatchScoreBreakdown,
    MetadataProvider, MusicBrainzClient, OnlineArtist, OnlineArtistCredit, OnlineMedia,
    OnlineRelease, OnlineReleaseGroup, OnlineTrack, ProviderError, RankedCandidateMatch,
    RateLimiter, StringNormalizer,
};
pub use queue::{PlaybackQueue, QueueItem};
pub use services::{LibraryService, LyricsService, PlaybackService, QueueService};
