//! Sonora Online Metadata, Search, Provider, and Matching Subsystem.

pub mod error;
pub mod matching;
pub mod models;
pub mod musicbrainz;
pub mod throttle;
pub mod traits;

#[cfg(test)]
mod tests;

pub use error::ProviderError;
pub use matching::{
    duration_score, jaro_winkler_similarity, levenshtein_distance, levenshtein_similarity,
    token_set_similarity, token_sort_similarity, DeterministicMatcher, StringNormalizer,
    THRESHOLD_HIGH, THRESHOLD_MEDIUM, WEIGHT_ALBUM, WEIGHT_ARTIST, WEIGHT_DURATION, WEIGHT_TITLE,
    WEIGHT_TRACK,
};
pub use models::{
    ArtistType, ConfidenceTier, ExternalLink, LocalTrackMetadata, MatchScoreBreakdown,
    OnlineArtist, OnlineArtistCredit, OnlineMedia, OnlineRelease, OnlineReleaseGroup, OnlineTrack,
    RankedCandidateMatch,
};
pub use musicbrainz::{MusicBrainzClient, DEFAULT_MUSICBRAINZ_BASE_URL, DEFAULT_USER_AGENT};
pub use throttle::RateLimiter;
pub use traits::MetadataProvider;
