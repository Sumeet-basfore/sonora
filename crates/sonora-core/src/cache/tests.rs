use crate::artwork::models::{ArtworkCandidate, ArtworkKind, ArtworkSourceType};
use crate::cache::OnlineCacheManager;
use crate::lyrics_online::models::{LyricsCandidate, LyricsSourceKind, LyricsSyncType};
use crate::metadata::models::{
    ConfidenceTier, MatchScoreBreakdown, OnlineArtist, RankedCandidateMatch,
};
use sonora_library::Database;
use std::sync::Arc;

#[test]
fn test_cache_metadata_entity_fresh_and_stale() {
    let db = Arc::new(Database::in_memory().expect("In-memory DB creation failed"));
    let cache = OnlineCacheManager::new(db);

    let artist = OnlineArtist {
        mbid: "a74b1b7f-71a5-4011-9441-d0b5e4122711".to_string(),
        name: "Radiohead".to_string(),
        sort_name: Some("Radiohead".to_string()),
        disambiguation: Some("UK rock band".to_string()),
        artist_type: Some(crate::metadata::models::ArtistType::Group),
        country: Some("GB".to_string()),
        biography: Some("Formed in Abingdon-on-Thames".to_string()),
        external_links: vec![],
    };

    // 1. Initially cache miss
    let miss = cache.get_artist_by_mbid(&artist.mbid, false);
    assert!(miss.is_none());

    // 2. Insert with standard TTL
    cache.set_artist(&artist, Some(3600));

    // 3. Cache hit (fresh)
    let hit = cache
        .get_artist_by_mbid(&artist.mbid, false)
        .expect("Expected cache hit");
    assert_eq!(hit.name, "Radiohead");
    assert_eq!(hit.country, Some("GB".to_string()));

    // 4. Test expired entry with negative / 0 offset
    let expired_artist = OnlineArtist {
        mbid: "expired_mbid".to_string(),
        name: "Expired Artist".to_string(),
        sort_name: None,
        disambiguation: None,
        artist_type: None,
        country: None,
        biography: None,
        external_links: vec![],
    };

    // Insert with 0 TTL (expires immediately at unixepoch())
    cache.set_artist(&expired_artist, Some(0));

    // Fresh lookup should miss because expires_at <= unixepoch()
    let fresh_lookup = cache.get_artist_by_mbid("expired_mbid", false);
    assert!(fresh_lookup.is_none());

    // Stale lookup should return the cached data
    let stale_lookup = cache.get_artist_by_mbid("expired_mbid", true);
    assert!(stale_lookup.is_some());
    assert_eq!(stale_lookup.unwrap().name, "Expired Artist");
}

#[test]
fn test_cache_ranked_metadata_candidates() {
    let db = Arc::new(Database::in_memory().expect("In-memory DB"));
    let cache = OnlineCacheManager::new(db);

    let fp = OnlineCacheManager::metadata_query_fingerprint(
        "Paranoid Android",
        Some("Radiohead"),
        Some("OK Computer"),
        Some(383000),
    );

    let candidate = RankedCandidateMatch {
        candidate_track: crate::metadata::models::OnlineTrack {
            recording_mbid: "rec_123".to_string(),
            release_mbid: Some("rel_123".to_string()),
            release_group_mbid: Some("rg_123".to_string()),
            position: Some(2),
            number: Some("2".to_string()),
            title: "Paranoid Android".to_string(),
            duration_ms: Some(383000),
            artist_credits: vec![crate::metadata::models::OnlineArtistCredit {
                artist_mbid: "art_123".to_string(),
                name: "Radiohead".to_string(),
                join_phrase: None,
            }],
            isrcs: vec![],
        },
        candidate_release: None,
        candidate_release_group: None,
        score_breakdown: MatchScoreBreakdown {
            total_score: 0.98,
            confidence_tier: ConfidenceTier::High,
            title_score: 1.0,
            artist_score: 1.0,
            album_score: 1.0,
            duration_score: 1.0,
            track_number_score: 1.0,
            is_exact_shortcut: false,
            shortcut_reason: None,
        },
    };

    cache.set_metadata_candidates(&fp, &[candidate], Some(3600));

    let hit = cache
        .get_metadata_candidates(&fp, false)
        .expect("Expected candidate cache hit");
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].candidate_track.title, "Paranoid Android");
    assert_eq!(hit[0].score_breakdown.confidence_tier, ConfidenceTier::High);
}

#[test]
fn test_cache_artwork_candidates() {
    let db = Arc::new(Database::in_memory().expect("In-memory DB"));
    let cache = OnlineCacheManager::new(db);

    let key = "art:rel1:rg1:art1";
    let candidate = ArtworkCandidate {
        id: "caa_1".to_string(),
        provider_name: "Cover Art Archive".to_string(),
        source_type: ArtworkSourceType::CoverArtArchive {
            release_mbid: Some("rel1".to_string()),
            release_group_mbid: None,
        },
        kind: ArtworkKind::FrontCover,
        original_url: "http://example.com/cover.jpg".to_string(),
        preview_thumbnail_url: "http://example.com/thumb.jpg".to_string(),
        width: 1200,
        height: 1200,
        format: "JPEG".to_string(),
        size_bytes: Some(102400),
        match_confidence: 1.0,
        is_canonical: true,
    };

    cache.set_artwork_candidates(key, "release", &[candidate], "coverartarchive", Some(3600));

    let hit = cache
        .get_artwork_candidates(key, false)
        .expect("Expected artwork cache hit");
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].kind, ArtworkKind::FrontCover);
}

#[test]
fn test_cache_lyrics_candidates() {
    let db = Arc::new(Database::in_memory().expect("In-memory DB"));
    let cache = OnlineCacheManager::new(db);

    let fp = OnlineCacheManager::lyrics_query_fingerprint(
        "No Surprises",
        Some("Radiohead"),
        Some("OK Computer"),
        Some(228.0),
    );

    let candidate = LyricsCandidate {
        candidate_id: "lrclib_100".to_string(),
        source_kind: LyricsSourceKind::LrclibPublicApi { id: 100 },
        provider_name: "LRCLIB".to_string(),
        sync_type: LyricsSyncType::LineSynced,
        track_name: "No Surprises".to_string(),
        artist_name: "Radiohead".to_string(),
        album_name: Some("OK Computer".to_string()),
        duration_seconds: 228.0,
        duration_delta_seconds: 0.0,
        match_confidence: 0.99,
        language_code: Some("en".to_string()),
        is_instrumental: false,
        raw_content: "[00:10.00] A heart that's full up like a landfill".to_string(),
    };

    cache.set_lyrics_candidates(&fp, &[candidate], Some(3600));

    let hit = cache
        .get_lyrics_candidates(&fp, false)
        .expect("Expected lyrics candidates hit");
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].sync_type, LyricsSyncType::LineSynced);
}
