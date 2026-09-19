//! Sonora Online Metadata Domain Models.
//!
//! Strongly-typed domain representations of music entities (Artists, Releases,
//! Release Groups, Recordings/Tracks) and matching candidate results.

use serde::{Deserialize, Serialize};

/// Classification of artist entity type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtistType {
    Person,
    Group,
    Orchestra,
    Choir,
    Character,
    Other,
}

/// External relational link for an artist or release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalLink {
    pub link_type: String,
    pub target_url: String,
}

/// Canonical artist credit with optional join phrase (e.g. " feat. ", " & ").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineArtistCredit {
    pub artist_mbid: String,
    pub name: String,
    pub join_phrase: Option<String>,
}

/// Online artist entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineArtist {
    pub mbid: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub artist_type: Option<ArtistType>,
    pub country: Option<String>,
    pub disambiguation: Option<String>,
    pub biography: Option<String>,
    pub external_links: Vec<ExternalLink>,
}

/// Online release group entity representing the higher-level musical work / album.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineReleaseGroup {
    pub mbid: String,
    pub title: String,
    pub primary_type: Option<String>,
    pub secondary_types: Vec<String>,
    pub first_release_date: Option<String>,
    pub artist_credits: Vec<OnlineArtistCredit>,
}

/// Online release entity representing a specific physical or digital pressing/issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineRelease {
    pub mbid: String,
    pub release_group_mbid: Option<String>,
    pub title: String,
    pub status: Option<String>,
    pub date: Option<String>,
    pub country: Option<String>,
    pub barcode: Option<String>,
    pub media_format: Option<String>,
    pub track_count: u32,
    pub media: Vec<OnlineMedia>,
    pub artist_credits: Vec<OnlineArtistCredit>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
}

/// Medium / Disc in a release (e.g., Disc 1, Disc 2, Vinyl Side A/B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineMedia {
    pub position: u32,
    pub format: Option<String>,
    pub title: Option<String>,
    pub track_count: u32,
    pub tracks: Vec<OnlineTrack>,
}

/// Online track / recording entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineTrack {
    pub recording_mbid: String,
    pub release_mbid: Option<String>,
    pub release_group_mbid: Option<String>,
    pub position: Option<u32>,
    pub number: Option<String>,
    pub title: String,
    pub duration_ms: Option<u64>,
    pub artist_credits: Vec<OnlineArtistCredit>,
    pub isrcs: Vec<String>,
}

/// Input local file metadata used for candidate query and matching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LocalTrackMetadata {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub duration_ms: u64,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub isrc: Option<String>,
    pub musicbrainz_track_id: Option<String>,
    pub musicbrainz_release_group_id: Option<String>,
}

/// Confidence classification tier for candidate scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceTier {
    /// Safe suggestion (score >= 0.90), eligible for safe 1-click batch application.
    High,
    /// Requires user confirmation (0.65 <= score < 0.90) via side-by-side diff.
    Medium,
    /// Weak/ambiguous match (score < 0.65), presents alternatives; manual choice required.
    Low,
}

/// Explainable breakdown of candidate match confidence score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchScoreBreakdown {
    pub total_score: f32,
    pub confidence_tier: ConfidenceTier,
    pub title_score: f32,
    pub artist_score: f32,
    pub album_score: f32,
    pub duration_score: f32,
    pub track_number_score: f32,
    pub is_exact_shortcut: bool,
    pub shortcut_reason: Option<String>,
}

/// Ranked online candidate with associated release context and match explanation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankedCandidateMatch {
    pub candidate_track: OnlineTrack,
    pub candidate_release: Option<OnlineRelease>,
    pub candidate_release_group: Option<OnlineReleaseGroup>,
    pub score_breakdown: MatchScoreBreakdown,
}
