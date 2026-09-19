//! Metadata matching, normalization, and confidence scoring.

pub mod matcher;
pub mod metrics;
pub mod normalizer;

pub use matcher::{
    DeterministicMatcher, THRESHOLD_HIGH, THRESHOLD_MEDIUM, WEIGHT_ALBUM, WEIGHT_ARTIST,
    WEIGHT_DURATION, WEIGHT_TITLE, WEIGHT_TRACK,
};
pub use metrics::{
    duration_score, jaro_winkler_similarity, levenshtein_distance, levenshtein_similarity,
    token_set_similarity, token_sort_similarity,
};
pub use normalizer::StringNormalizer;
