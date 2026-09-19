//! Deterministic metadata matcher and candidate ranker.

use crate::metadata::matching::metrics::{
    duration_score, jaro_winkler_similarity, token_set_similarity, token_sort_similarity,
};
use crate::metadata::matching::normalizer::StringNormalizer;
use crate::metadata::models::{
    ConfidenceTier, LocalTrackMetadata, MatchScoreBreakdown, OnlineRelease, OnlineReleaseGroup,
    OnlineTrack, RankedCandidateMatch,
};

/// Weight parameters defined in docs/27-metadata-matching.md
pub const WEIGHT_TITLE: f32 = 0.35;
pub const WEIGHT_ARTIST: f32 = 0.25;
pub const WEIGHT_ALBUM: f32 = 0.20;
pub const WEIGHT_DURATION: f32 = 0.15;
pub const WEIGHT_TRACK: f32 = 0.05;

/// Thresholds for confidence tiers
pub const THRESHOLD_HIGH: f32 = 0.90;
pub const THRESHOLD_MEDIUM: f32 = 0.65;

/// Deterministic, explainable candidate matcher.
pub struct DeterministicMatcher;

impl DeterministicMatcher {
    /// Score a single online candidate against local track metadata.
    pub fn score_candidate(
        local: &LocalTrackMetadata,
        candidate_track: &OnlineTrack,
        candidate_release: Option<&OnlineRelease>,
        candidate_release_group: Option<&OnlineReleaseGroup>,
    ) -> MatchScoreBreakdown {
        // 1. Exact MBID Shortcut Check (Score = 1.00)
        if let Some(ref local_mbid) = local.musicbrainz_track_id {
            if !local_mbid.is_empty() && local_mbid == &candidate_track.recording_mbid {
                return MatchScoreBreakdown {
                    total_score: 1.00,
                    confidence_tier: ConfidenceTier::High,
                    title_score: 1.00,
                    artist_score: 1.00,
                    album_score: 1.00,
                    duration_score: 1.00,
                    track_number_score: 1.00,
                    is_exact_shortcut: true,
                    shortcut_reason: Some("Exact MusicBrainz Recording ID Match".to_string()),
                };
            }
        }

        // 2. Exact ISRC Shortcut Check (Score = 1.00)
        if let Some(ref local_isrc) = local.isrc {
            let clean_local_isrc = local_isrc.trim().to_uppercase();
            if !clean_local_isrc.is_empty()
                && candidate_track
                    .isrcs
                    .iter()
                    .any(|i| i.trim().to_uppercase() == clean_local_isrc)
            {
                return MatchScoreBreakdown {
                    total_score: 1.00,
                    confidence_tier: ConfidenceTier::High,
                    title_score: 1.00,
                    artist_score: 1.00,
                    album_score: 1.00,
                    duration_score: 1.00,
                    track_number_score: 1.00,
                    is_exact_shortcut: true,
                    shortcut_reason: Some("Exact ISRC Match".to_string()),
                };
            }
        }

        // 3. Title Scoring (0.35 weight)
        let (local_clean_title, local_version) =
            StringNormalizer::extract_version_info(&local.title);
        let (cand_clean_title, cand_version) =
            StringNormalizer::extract_version_info(&candidate_track.title);

        let clean_title_sim = jaro_winkler_similarity(&local_clean_title, &cand_clean_title);
        let raw_title_sim = jaro_winkler_similarity(
            &StringNormalizer::normalize(&local.title),
            &StringNormalizer::normalize(&candidate_track.title),
        );
        let mut title_score = clean_title_sim.max(raw_title_sim);

        // Version mismatch penalty (e.g. one is "Live" and the other is not)
        if local_version.is_some() != cand_version.is_some()
            && (local_version.as_deref() == Some("live") || cand_version.as_deref() == Some("live"))
        {
            title_score = (title_score - 0.20).max(0.0);
        }

        // 4. Artist Scoring (0.25 weight)
        let local_artist_raw = local.artist.as_deref().unwrap_or_default();
        let (local_primary_artist, local_featured) =
            StringNormalizer::extract_featured_artists(local_artist_raw);

        let cand_artist_raw = candidate_track
            .artist_credits
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<&str>>()
            .join(" ");

        let (cand_primary_artist, cand_featured) = if !candidate_track.artist_credits.is_empty() {
            let (p, mut f) =
                StringNormalizer::extract_featured_artists(&candidate_track.artist_credits[0].name);
            let mut other_credits: Vec<String> = candidate_track
                .artist_credits
                .iter()
                .skip(1)
                .map(|c| StringNormalizer::normalize(&c.name))
                .filter(|s| !s.is_empty())
                .collect();
            f.append(&mut other_credits);
            (p, f)
        } else {
            StringNormalizer::extract_featured_artists(&cand_artist_raw)
        };

        let primary_sim = token_set_similarity(&local_primary_artist, &cand_primary_artist).max(
            jaro_winkler_similarity(&local_primary_artist, &cand_primary_artist),
        );

        // Featured artist bonus or comparison
        let featured_match = if !local_featured.is_empty() && !cand_featured.is_empty() {
            let local_feat_str = local_featured.join(" ");
            let cand_feat_str = cand_featured.join(" ");
            token_set_similarity(&local_feat_str, &cand_feat_str)
                .max(jaro_winkler_similarity(&local_feat_str, &cand_feat_str))
        } else if local_featured.is_empty() && cand_featured.is_empty() {
            1.0
        } else {
            0.5
        };

        let full_artist_raw_sim = jaro_winkler_similarity(
            &StringNormalizer::normalize(local_artist_raw),
            &StringNormalizer::normalize(&cand_artist_raw),
        );

        let artist_score = if local_artist_raw.is_empty() {
            0.50
        } else {
            let weighted = primary_sim * 0.85 + featured_match * 0.15;
            weighted.max(full_artist_raw_sim).min(1.0)
        };

        // 5. Album Scoring (0.20 weight)
        let album_score = if let Some(ref local_album) = local.album {
            let norm_local_album = StringNormalizer::normalize(local_album);

            let release_sim = candidate_release.map(|r| {
                let norm = StringNormalizer::normalize(&r.title);
                token_sort_similarity(&norm_local_album, &norm)
                    .max(jaro_winkler_similarity(&norm_local_album, &norm))
            });

            let group_sim = candidate_release_group.map(|rg| {
                let norm = StringNormalizer::normalize(&rg.title);
                token_sort_similarity(&norm_local_album, &norm)
                    .max(jaro_winkler_similarity(&norm_local_album, &norm))
            });

            match (release_sim, group_sim) {
                (Some(s1), Some(s2)) => s1.max(s2),
                (Some(s), None) | (None, Some(s)) => s,
                (None, None) => 0.50, // Neutral when candidate has no release info
            }
        } else {
            // Neutral score if local file has no album tag
            0.50
        };

        // 6. Duration Scoring (0.15 weight)
        let cand_duration = candidate_track.duration_ms.unwrap_or(0);
        let duration_score = duration_score(local.duration_ms, cand_duration);

        // 7. Track Number / Disc Number Scoring (0.05 weight)
        let track_number_score = match (local.track_number, candidate_track.position) {
            (Some(local_pos), Some(cand_pos)) => {
                if local_pos == cand_pos {
                    1.00
                } else {
                    0.00
                }
            }
            (None, _) => 0.50,
            _ => 0.00,
        };

        // Composite Weighted Score Formula
        let total_score = (WEIGHT_TITLE * title_score)
            + (WEIGHT_ARTIST * artist_score)
            + (WEIGHT_ALBUM * album_score)
            + (WEIGHT_DURATION * duration_score)
            + (WEIGHT_TRACK * track_number_score);

        let total_score = (total_score * 100.0).round() / 100.0;
        let total_score = total_score.clamp(0.0, 1.0);

        let confidence_tier = if total_score >= THRESHOLD_HIGH {
            ConfidenceTier::High
        } else if total_score >= THRESHOLD_MEDIUM {
            ConfidenceTier::Medium
        } else {
            ConfidenceTier::Low
        };

        MatchScoreBreakdown {
            total_score,
            confidence_tier,
            title_score,
            artist_score,
            album_score,
            duration_score,
            track_number_score,
            is_exact_shortcut: false,
            shortcut_reason: None,
        }
    }

    /// Rank a list of candidate tracks and return ordered matches with breakdowns.
    pub fn rank_candidates(
        local: &LocalTrackMetadata,
        candidates: Vec<(
            OnlineTrack,
            Option<OnlineRelease>,
            Option<OnlineReleaseGroup>,
        )>,
    ) -> Vec<RankedCandidateMatch> {
        let mut matches: Vec<RankedCandidateMatch> = candidates
            .into_iter()
            .map(|(track, release, release_group)| {
                let breakdown =
                    Self::score_candidate(local, &track, release.as_ref(), release_group.as_ref());
                RankedCandidateMatch {
                    candidate_track: track,
                    candidate_release: release,
                    candidate_release_group: release_group,
                    score_breakdown: breakdown,
                }
            })
            .collect();

        // Sort descending by total score
        matches.sort_by(|a, b| {
            b.score_breakdown
                .total_score
                .partial_cmp(&a.score_breakdown.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        matches
    }
}
