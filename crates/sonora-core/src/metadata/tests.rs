//! Comprehensive unit tests for metadata normalization, metrics, matcher,
//! rate limiter, and provider client.

use crate::metadata::matching::metrics::{
    duration_score, jaro_winkler_similarity, levenshtein_distance, levenshtein_similarity,
    token_set_similarity, token_sort_similarity,
};
use crate::metadata::matching::normalizer::StringNormalizer;
use crate::metadata::models::{
    ConfidenceTier, LocalTrackMetadata, OnlineArtistCredit, OnlineMedia, OnlineRelease,
    OnlineReleaseGroup, OnlineTrack,
};
use crate::metadata::musicbrainz::client::MusicBrainzClient;
use crate::metadata::throttle::RateLimiter;
use crate::metadata::traits::MetadataProvider;
use crate::metadata::DeterministicMatcher;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::Instant;

// ========================================================================
// 1. Normalizer Tests
// ========================================================================

#[test]
fn test_normalizer_diacritics_and_unicode() {
    assert_eq!(StringNormalizer::normalize("Björk"), "bjork");
    assert_eq!(StringNormalizer::normalize("Motörhead"), "motorhead");
    assert_eq!(StringNormalizer::normalize("Sigur Rós"), "sigur ros");
    assert_eq!(StringNormalizer::normalize("Café Tacvba"), "cafe tacvba");
    assert_eq!(
        StringNormalizer::normalize("Cœur de pirate"),
        "coeur de pirate"
    );
    assert_eq!(StringNormalizer::normalize("Kælan Mikla"), "kaelan mikla");
    assert_eq!(StringNormalizer::normalize("Strauß"), "strauss");
}

#[test]
fn test_normalizer_punctuation_and_whitespace() {
    assert_eq!(
        StringNormalizer::normalize("  The Dark Side   of the Moon (2011 Remastered)  "),
        "the dark side of the moon 2011 remastered"
    );
    assert_eq!(
        StringNormalizer::normalize("AC/DC - Back In Black!"),
        "ac dc back in black"
    );
    assert_eq!(
        StringNormalizer::normalize("Rock & Roll: Part 1..."),
        "rock roll part 1"
    );
}

#[test]
fn test_normalizer_feature_artist_extraction() {
    let (main, feat) =
        StringNormalizer::extract_featured_artists("Gorillaz feat. Del The Funky Homosapien");
    assert_eq!(main, "gorillaz");
    assert_eq!(feat, vec!["del the funky homosapien"]);

    let (main, feat) = StringNormalizer::extract_featured_artists(
        "Daft Punk ft. Pharrell Williams & Nile Rodgers",
    );
    assert_eq!(main, "daft punk");
    assert_eq!(feat, vec!["pharrell williams", "nile rodgers"]);

    let (main, feat) = StringNormalizer::extract_featured_artists("Pink Floyd");
    assert_eq!(main, "pink floyd");
    assert!(feat.is_empty());
}

#[test]
fn test_normalizer_version_extraction() {
    let (title, ver) = StringNormalizer::extract_version_info("Time (2011 Remastered)");
    assert_eq!(title, "time 2011");
    assert_eq!(ver, Some("remastered".to_string()));

    let (title, ver) = StringNormalizer::extract_version_info("Comfortably Numb (Live at Pompeii)");
    assert_eq!(title, "comfortably numb pompeii");
    assert_eq!(ver, Some("live at".to_string()));

    let (title, ver) = StringNormalizer::extract_version_info("Money");
    assert_eq!(title, "money");
    assert_eq!(ver, None);
}

// ========================================================================
// 2. Metrics Tests
// ========================================================================

#[test]
fn test_jaro_winkler_similarity() {
    // Exact match
    assert!((jaro_winkler_similarity("abbey road", "abbey road") - 1.0).abs() < 1e-4);

    // Close match with shared prefix
    let sim = jaro_winkler_similarity("the beatles", "the beattles");
    assert!(sim > 0.90);

    // Different strings
    let sim_diff = jaro_winkler_similarity("pink floyd", "led zeppelin");
    assert!(sim_diff < 0.50);

    // Empty strings
    assert_eq!(jaro_winkler_similarity("", ""), 1.0);
    assert_eq!(jaro_winkler_similarity("abc", ""), 0.0);
}

#[test]
fn test_levenshtein_distance_and_similarity() {
    assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    assert_eq!(levenshtein_distance("rosettacode", "raisethysword"), 8);

    let sim = levenshtein_similarity("dark side", "dark side");
    assert!((sim - 1.0).abs() < 1e-4);

    let sim = levenshtein_similarity("hello", "helo");
    assert!((sim - 0.80).abs() < 1e-4);
}

#[test]
fn test_token_similarity() {
    // Token sort handles transposed words
    let sim_sort = token_sort_similarity("side dark the of moon", "the dark side of the moon");
    assert!(sim_sort > 0.90);

    // Token set handles subset of words
    let sim_set = token_set_similarity("pink floyd the wall", "pink floyd");
    assert!(sim_set > 0.85);
}

#[test]
fn test_duration_scoring_curve() {
    // Delta <= 1.0s -> 1.00
    assert!((duration_score(240_000, 240_500) - 1.00).abs() < 1e-3);
    assert!((duration_score(240_000, 241_000) - 1.00).abs() < 1e-3);

    // Delta = 3.0s -> 0.60
    assert!((duration_score(240_000, 243_000) - 0.60).abs() < 1e-2);

    // Delta = 5.0s -> 0.25
    assert!((duration_score(240_000, 245_000) - 0.25).abs() < 1e-2);

    // Delta = 10.0s -> 0.00
    assert!((duration_score(240_000, 250_000) - 0.00).abs() < 1e-2);

    // Delta > 10.0s -> 0.00
    assert!((duration_score(240_000, 260_000) - 0.00).abs() < 1e-3);
}

// ========================================================================
// 3. Matcher & Scoring Tests
// ========================================================================

fn sample_candidate_track() -> OnlineTrack {
    OnlineTrack {
        recording_mbid: "rec-1234".to_string(),
        release_mbid: Some("rel-5678".to_string()),
        release_group_mbid: Some("rg-9999".to_string()),
        position: Some(1),
        number: Some("A1".to_string()),
        title: "Come Together".to_string(),
        duration_ms: Some(259_000),
        artist_credits: vec![OnlineArtistCredit {
            artist_mbid: "art-1111".to_string(),
            name: "The Beatles".to_string(),
            join_phrase: None,
        }],
        isrcs: vec!["GBAYE0601498".to_string()],
    }
}

fn sample_candidate_release() -> OnlineRelease {
    OnlineRelease {
        mbid: "rel-5678".to_string(),
        release_group_mbid: Some("rg-9999".to_string()),
        title: "Abbey Road".to_string(),
        status: Some("Official".to_string()),
        date: Some("1969-09-26".to_string()),
        country: Some("GB".to_string()),
        barcode: Some("077774644624".to_string()),
        media_format: Some("12\" Vinyl".to_string()),
        track_count: 17,
        media: vec![OnlineMedia {
            position: 1,
            format: Some("Vinyl".to_string()),
            title: None,
            track_count: 17,
            tracks: vec![],
        }],
        artist_credits: vec![OnlineArtistCredit {
            artist_mbid: "art-1111".to_string(),
            name: "The Beatles".to_string(),
            join_phrase: None,
        }],
        label: Some("Apple Records".to_string()),
        catalog_number: Some("PCS 7088".to_string()),
    }
}

fn sample_candidate_release_group() -> OnlineReleaseGroup {
    OnlineReleaseGroup {
        mbid: "rg-9999".to_string(),
        title: "Abbey Road".to_string(),
        primary_type: Some("Album".to_string()),
        secondary_types: vec![],
        first_release_date: Some("1969-09-26".to_string()),
        artist_credits: vec![OnlineArtistCredit {
            artist_mbid: "art-1111".to_string(),
            name: "The Beatles".to_string(),
            join_phrase: None,
        }],
    }
}

#[test]
fn test_matcher_exact_mbid_shortcut() {
    let local = LocalTrackMetadata {
        title: "Different Title in Local Tags".to_string(),
        artist: Some("Unknown Artist".to_string()),
        album: None,
        album_artist: None,
        duration_ms: 100_000,
        track_number: None,
        disc_number: None,
        year: None,
        isrc: None,
        musicbrainz_track_id: Some("rec-1234".to_string()),
        musicbrainz_release_group_id: None,
    };

    let candidate = sample_candidate_track();
    let breakdown = DeterministicMatcher::score_candidate(&local, &candidate, None, None);

    assert_eq!(breakdown.total_score, 1.00);
    assert_eq!(breakdown.confidence_tier, ConfidenceTier::High);
    assert!(breakdown.is_exact_shortcut);
    assert!(breakdown.shortcut_reason.unwrap().contains("MusicBrainz"));
}

#[test]
fn test_matcher_exact_isrc_shortcut() {
    let local = LocalTrackMetadata {
        title: "Come Together".to_string(),
        artist: Some("The Beatles".to_string()),
        album: None,
        album_artist: None,
        duration_ms: 259_000,
        track_number: None,
        disc_number: None,
        year: None,
        isrc: Some("GBAYE0601498".to_string()),
        musicbrainz_track_id: None,
        musicbrainz_release_group_id: None,
    };

    let candidate = sample_candidate_track();
    let breakdown = DeterministicMatcher::score_candidate(&local, &candidate, None, None);

    assert_eq!(breakdown.total_score, 1.00);
    assert_eq!(breakdown.confidence_tier, ConfidenceTier::High);
    assert!(breakdown.is_exact_shortcut);
    assert!(breakdown.shortcut_reason.unwrap().contains("ISRC"));
}

#[test]
fn test_matcher_high_confidence_fuzzy_match() {
    let local = LocalTrackMetadata {
        title: "Come Together (2019 Mix)".to_string(),
        artist: Some("The Beatles".to_string()),
        album: Some("Abbey Road (Super Deluxe)".to_string()),
        album_artist: Some("The Beatles".to_string()),
        duration_ms: 259_200, // 0.2s delta
        track_number: Some(1),
        disc_number: Some(1),
        year: Some(1969),
        isrc: None,
        musicbrainz_track_id: None,
        musicbrainz_release_group_id: None,
    };

    let track = sample_candidate_track();
    let release = sample_candidate_release();
    let release_group = sample_candidate_release_group();

    let breakdown =
        DeterministicMatcher::score_candidate(&local, &track, Some(&release), Some(&release_group));

    assert!(
        breakdown.total_score >= 0.90,
        "Expected high score, got: {}",
        breakdown.total_score
    );
    assert_eq!(breakdown.confidence_tier, ConfidenceTier::High);
    assert!(!breakdown.is_exact_shortcut);
}

#[test]
fn test_matcher_low_confidence_mismatch() {
    let local = LocalTrackMetadata {
        title: "Stairway to Heaven".to_string(),
        artist: Some("Led Zeppelin".to_string()),
        album: Some("Led Zeppelin IV".to_string()),
        album_artist: Some("Led Zeppelin".to_string()),
        duration_ms: 482_000,
        track_number: Some(4),
        disc_number: Some(1),
        year: Some(1971),
        isrc: None,
        musicbrainz_track_id: None,
        musicbrainz_release_group_id: None,
    };

    let track = sample_candidate_track(); // Come Together / Beatles
    let release = sample_candidate_release();
    let release_group = sample_candidate_release_group();

    let breakdown =
        DeterministicMatcher::score_candidate(&local, &track, Some(&release), Some(&release_group));

    assert!(
        breakdown.total_score < 0.65,
        "Expected low score, got: {}",
        breakdown.total_score
    );
    assert_eq!(breakdown.confidence_tier, ConfidenceTier::Low);
}

#[test]
fn test_matcher_rank_candidates() {
    let local = LocalTrackMetadata {
        title: "Come Together".to_string(),
        artist: Some("The Beatles".to_string()),
        album: Some("Abbey Road".to_string()),
        album_artist: Some("The Beatles".to_string()),
        duration_ms: 259_000,
        track_number: Some(1),
        disc_number: Some(1),
        year: Some(1969),
        isrc: None,
        musicbrainz_track_id: None,
        musicbrainz_release_group_id: None,
    };

    let good_track = sample_candidate_track();
    let mut bad_track = sample_candidate_track();
    bad_track.title = "Something".to_string();
    bad_track.recording_mbid = "rec-bad".to_string();
    bad_track.isrcs = vec![];
    bad_track.position = Some(2);

    let release = sample_candidate_release();
    let release_group = sample_candidate_release_group();

    let candidates = vec![
        (
            bad_track,
            Some(release.clone()),
            Some(release_group.clone()),
        ),
        (good_track, Some(release), Some(release_group)),
    ];

    let ranked = DeterministicMatcher::rank_candidates(&local, candidates);

    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].candidate_track.title, "Come Together");
    assert!(ranked[0].score_breakdown.total_score > ranked[1].score_breakdown.total_score);
}

// ========================================================================
// 4. Rate Limiter Tests
// ========================================================================

#[tokio::test]
async fn test_rate_limiter_interval() {
    let limiter = RateLimiter::new(Duration::from_millis(50));

    let start = Instant::now();
    limiter.acquire().await;
    limiter.acquire().await;
    limiter.acquire().await;
    let elapsed = start.elapsed();

    // 3 calls with 50ms interval should take >= 100ms
    assert!(
        elapsed >= Duration::from_millis(90),
        "Expected elapsed >= 90ms, got {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_rate_limiter_delay_injection() {
    let limiter = RateLimiter::new(Duration::from_millis(10));
    limiter.delay_until(Duration::from_millis(80)).await;

    let start = Instant::now();
    limiter.acquire().await;
    let elapsed = start.elapsed();

    assert!(
        elapsed >= Duration::from_millis(70),
        "Expected elapsed >= 70ms due to backoff delay, got {:?}",
        elapsed
    );
}

// ========================================================================
// 5. Mock HTTP Server & MusicBrainz Client Tests
// ========================================================================

async fn start_mock_mb_server(
    status_code: u16,
    body: &'static str,
    headers: Vec<(&'static str, &'static str)>,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 2048];
            let _ = socket.read(&mut buf).await;

            let mut header_lines = String::new();
            for (k, v) in headers {
                header_lines.push_str(&format!("{k}: {v}\r\n"));
            }

            let response = format!(
                    "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
                    status_code,
                    body.len(),
                    header_lines,
                    body
                );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.flush().await;
        }
    });

    (format!("http://{}", addr), handle)
}

#[tokio::test]
async fn test_musicbrainz_client_artist_search() {
    let fixture_json = r#"{
            "count": 1,
            "artists": [
                {
                    "id": "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d",
                    "type": "Group",
                    "name": "The Beatles",
                    "sort-name": "Beatles, The",
                    "country": "GB",
                    "disambiguation": "legendary rock band"
                }
            ]
        }"#;

    let (server_url, _handle) = start_mock_mb_server(200, fixture_json, vec![]).await;

    let client = MusicBrainzClient::with_config(
        server_url,
        "Sonora/Test".to_string(),
        Duration::from_secs(2),
        RateLimiter::new(Duration::from_millis(0)),
    );

    let artists = client.search_artists("The Beatles", 10).await.unwrap();
    assert_eq!(artists.len(), 1);
    assert_eq!(artists[0].name, "The Beatles");
    assert_eq!(artists[0].mbid, "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d");
    assert_eq!(
        artists[0].disambiguation.as_deref(),
        Some("legendary rock band")
    );
}

#[tokio::test]
async fn test_musicbrainz_client_recording_search() {
    let fixture_json = r#"{
            "count": 1,
            "recordings": [
                {
                    "id": "c1f7b03b-d368-45e0-94e8-f29e2f494578",
                    "title": "Come Together",
                    "length": 259000,
                    "isrcs": ["GBAYE0601498"],
                    "artist-credit": [
                        {
                            "name": "The Beatles",
                            "artist": {
                                "id": "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d",
                                "name": "The Beatles"
                            }
                        }
                    ]
                }
            ]
        }"#;

    let (server_url, _handle) = start_mock_mb_server(200, fixture_json, vec![]).await;

    let client = MusicBrainzClient::with_config(
        server_url,
        "Sonora/Test".to_string(),
        Duration::from_secs(2),
        RateLimiter::new(Duration::from_millis(0)),
    );

    let tracks = client.search_recordings("Come Together", 10).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "Come Together");
    assert_eq!(tracks[0].duration_ms, Some(259000));
    assert_eq!(tracks[0].isrcs, vec!["GBAYE0601498"]);
    assert_eq!(tracks[0].artist_credits[0].name, "The Beatles");
}

#[tokio::test]
async fn test_musicbrainz_client_http_429_rate_limit() {
    let (server_url, _handle) = start_mock_mb_server(
        429,
        r#"{"error": "Your requests are being throttled"}"#,
        vec![("Retry-After", "3")],
    )
    .await;

    let client = MusicBrainzClient::with_config(
        server_url,
        "Sonora/Test".to_string(),
        Duration::from_secs(2),
        RateLimiter::new(Duration::from_millis(0)),
    );

    let result = client.search_recordings("Come Together", 10).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        crate::metadata::ProviderError::RateLimited { retry_after_secs } => {
            assert_eq!(retry_after_secs, Some(3));
        }
        other => panic!("Expected RateLimited error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_musicbrainz_client_http_404_not_found() {
    let (server_url, _handle) =
        start_mock_mb_server(404, r#"{"error": "Not Found"}"#, vec![]).await;

    let client = MusicBrainzClient::with_config(
        server_url,
        "Sonora/Test".to_string(),
        Duration::from_secs(2),
        RateLimiter::new(Duration::from_millis(0)),
    );

    let result = client.get_artist_by_mbid("non-existent-mbid").await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::metadata::ProviderError::NotFound
    ));
}

#[tokio::test]
async fn test_musicbrainz_client_malformed_json() {
    let (server_url, _handle) = start_mock_mb_server(200, "<html>Not JSON</html>", vec![]).await;

    let client = MusicBrainzClient::with_config(
        server_url,
        "Sonora/Test".to_string(),
        Duration::from_secs(2),
        RateLimiter::new(Duration::from_millis(0)),
    );

    let result = client.search_artists("Beatles", 5).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::metadata::ProviderError::Parse(_)
    ));
}
