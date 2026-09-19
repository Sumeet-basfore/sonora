//! Integration tests for Sonora v0.2 Online Metadata Provider & Deterministic Matcher.

#[cfg(test)]
mod tests {
    use sonora_core::metadata::{
        ConfidenceTier, DeterministicMatcher, LocalTrackMetadata, MetadataProvider,
        MusicBrainzClient, OnlineArtistCredit, OnlineMedia, OnlineRelease, OnlineReleaseGroup,
        OnlineTrack, ProviderError, RateLimiter,
    };
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio::time::Instant;

    // ========================================================================
    // Helper Fixtures
    // ========================================================================

    fn sample_beatles_recording() -> OnlineTrack {
        OnlineTrack {
            recording_mbid: "c1f7b03b-d368-45e0-94e8-f29e2f494578".to_string(),
            release_mbid: Some("6238bfa0-75ff-4204-8b64-28b9d883015a".to_string()),
            release_group_mbid: Some("f5093c06-23e3-404f-aeaa-40f72885ee3a".to_string()),
            position: Some(1),
            number: Some("1".to_string()),
            title: "Come Together".to_string(),
            duration_ms: Some(259000),
            artist_credits: vec![OnlineArtistCredit {
                artist_mbid: "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d".to_string(),
                name: "The Beatles".to_string(),
                join_phrase: None,
            }],
            isrcs: vec!["GBAYE0601498".to_string()],
        }
    }

    fn sample_beatles_release() -> OnlineRelease {
        OnlineRelease {
            mbid: "6238bfa0-75ff-4204-8b64-28b9d883015a".to_string(),
            release_group_mbid: Some("f5093c06-23e3-404f-aeaa-40f72885ee3a".to_string()),
            title: "Abbey Road".to_string(),
            status: Some("Official".to_string()),
            date: Some("1969-09-26".to_string()),
            country: Some("GB".to_string()),
            barcode: Some("077774644624".to_string()),
            media_format: Some("CD".to_string()),
            track_count: 17,
            media: vec![OnlineMedia {
                position: 1,
                format: Some("CD".to_string()),
                title: None,
                track_count: 17,
                tracks: vec![],
            }],
            artist_credits: vec![OnlineArtistCredit {
                artist_mbid: "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d".to_string(),
                name: "The Beatles".to_string(),
                join_phrase: None,
            }],
            label: Some("Apple Records".to_string()),
            catalog_number: Some("CDP 7 46446 2".to_string()),
        }
    }

    fn sample_beatles_release_group() -> OnlineReleaseGroup {
        OnlineReleaseGroup {
            mbid: "f5093c06-23e3-404f-aeaa-40f72885ee3a".to_string(),
            title: "Abbey Road".to_string(),
            primary_type: Some("Album".to_string()),
            secondary_types: vec![],
            first_release_date: Some("1969-09-26".to_string()),
            artist_credits: vec![OnlineArtistCredit {
                artist_mbid: "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d".to_string(),
                name: "The Beatles".to_string(),
                join_phrase: None,
            }],
        }
    }

    // ========================================================================
    // Integration Tests: Matching Cases
    // ========================================================================

    #[test]
    fn test_integration_exact_mbid_match() {
        let local = LocalTrackMetadata {
            title: "Local Track 01".to_string(),
            artist: Some("Local Artist".to_string()),
            album: None,
            album_artist: None,
            duration_ms: 200_000,
            track_number: Some(1),
            disc_number: Some(1),
            year: None,
            isrc: None,
            musicbrainz_track_id: Some("c1f7b03b-d368-45e0-94e8-f29e2f494578".to_string()),
            musicbrainz_release_group_id: None,
        };

        let candidate = sample_beatles_recording();
        let breakdown = DeterministicMatcher::score_candidate(&local, &candidate, None, None);

        assert_eq!(breakdown.total_score, 1.00);
        assert_eq!(breakdown.confidence_tier, ConfidenceTier::High);
        assert!(breakdown.is_exact_shortcut);
        assert_eq!(
            breakdown.shortcut_reason.as_deref(),
            Some("Exact MusicBrainz Recording ID Match")
        );
    }

    #[test]
    fn test_integration_exact_isrc_match() {
        let local = LocalTrackMetadata {
            title: "Come Together".to_string(),
            artist: Some("The Beatles".to_string()),
            album: Some("Abbey Road".to_string()),
            album_artist: Some("The Beatles".to_string()),
            duration_ms: 259_000,
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(1969),
            isrc: Some("GBAYE0601498".to_string()),
            musicbrainz_track_id: None,
            musicbrainz_release_group_id: None,
        };

        let candidate = sample_beatles_recording();
        let breakdown = DeterministicMatcher::score_candidate(&local, &candidate, None, None);

        assert_eq!(breakdown.total_score, 1.00);
        assert_eq!(breakdown.confidence_tier, ConfidenceTier::High);
        assert!(breakdown.is_exact_shortcut);
        assert_eq!(
            breakdown.shortcut_reason.as_deref(),
            Some("Exact ISRC Match")
        );
    }

    #[test]
    fn test_integration_full_release_and_group_candidate_ranking() {
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

        let recording = sample_beatles_recording();
        let release = sample_beatles_release();
        let release_group = sample_beatles_release_group();

        let candidates = vec![(recording, Some(release), Some(release_group))];

        let ranked = DeterministicMatcher::rank_candidates(&local, candidates);
        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].score_breakdown.total_score >= 0.95);
        assert_eq!(
            ranked[0].score_breakdown.confidence_tier,
            ConfidenceTier::High
        );
        assert_eq!(
            ranked[0].candidate_release.as_ref().unwrap().title,
            "Abbey Road"
        );
        assert_eq!(
            ranked[0].candidate_release_group.as_ref().unwrap().title,
            "Abbey Road"
        );
    }

    #[test]
    fn test_integration_punctuation_and_diacritics_matching() {
        // Local file with diacritics and punctuation differences
        let local = LocalTrackMetadata {
            title: "Jóga - 2019 Master!".to_string(),
            artist: Some("Björk".to_string()),
            album: Some("Homogenic: Special Edition".to_string()),
            album_artist: Some("Björk".to_string()),
            duration_ms: 305_500, // 0.5s delta
            track_number: Some(2),
            disc_number: Some(1),
            year: Some(1997),
            isrc: None,
            musicbrainz_track_id: None,
            musicbrainz_release_group_id: None,
        };

        let candidate_track = OnlineTrack {
            recording_mbid: "bjork-rec-1".to_string(),
            release_mbid: Some("bjork-rel-1".to_string()),
            release_group_mbid: Some("bjork-rg-1".to_string()),
            position: Some(2),
            number: Some("2".to_string()),
            title: "Joga".to_string(),
            duration_ms: Some(305_000),
            artist_credits: vec![OnlineArtistCredit {
                artist_mbid: "bjork-art-1".to_string(),
                name: "Bjork".to_string(),
                join_phrase: None,
            }],
            isrcs: vec![],
        };

        let candidate_release = OnlineRelease {
            mbid: "bjork-rel-1".to_string(),
            release_group_mbid: Some("bjork-rg-1".to_string()),
            title: "Homogenic".to_string(),
            status: Some("Official".to_string()),
            date: Some("1997-09-22".to_string()),
            country: Some("IS".to_string()),
            barcode: None,
            media_format: Some("CD".to_string()),
            track_count: 10,
            media: vec![],
            artist_credits: vec![],
            label: Some("One Little Indian".to_string()),
            catalog_number: None,
        };

        let candidate_rg = OnlineReleaseGroup {
            mbid: "bjork-rg-1".to_string(),
            title: "Homogenic".to_string(),
            primary_type: Some("Album".to_string()),
            secondary_types: vec![],
            first_release_date: Some("1997-09-22".to_string()),
            artist_credits: vec![],
        };

        let breakdown = DeterministicMatcher::score_candidate(
            &local,
            &candidate_track,
            Some(&candidate_release),
            Some(&candidate_rg),
        );

        assert!(
            breakdown.total_score >= 0.90,
            "Expected score >= 0.90, got {}",
            breakdown.total_score
        );
        assert!(breakdown.artist_score > 0.95);
        assert!(breakdown.title_score > 0.80);
        assert_eq!(breakdown.confidence_tier, ConfidenceTier::High);
    }

    #[test]
    fn test_integration_feature_artist_handling() {
        let local = LocalTrackMetadata {
            title: "Feel Good Inc.".to_string(),
            artist: Some("Gorillaz feat. De La Soul".to_string()),
            album: Some("Demon Days".to_string()),
            album_artist: Some("Gorillaz".to_string()),
            duration_ms: 223_000,
            track_number: Some(6),
            disc_number: Some(1),
            year: Some(2005),
            isrc: None,
            musicbrainz_track_id: None,
            musicbrainz_release_group_id: None,
        };

        let candidate_track = OnlineTrack {
            recording_mbid: "gorillaz-rec".to_string(),
            release_mbid: None,
            release_group_mbid: None,
            position: Some(6),
            number: Some("6".to_string()),
            title: "Feel Good Inc".to_string(),
            duration_ms: Some(223_100),
            artist_credits: vec![
                OnlineArtistCredit {
                    artist_mbid: "gorillaz-art".to_string(),
                    name: "Gorillaz".to_string(),
                    join_phrase: Some(" feat. ".to_string()),
                },
                OnlineArtistCredit {
                    artist_mbid: "delasoul-art".to_string(),
                    name: "De La Soul".to_string(),
                    join_phrase: None,
                },
            ],
            isrcs: vec![],
        };

        let candidate_rg = OnlineReleaseGroup {
            mbid: "gorillaz-rg-demon-days".to_string(),
            title: "Demon Days".to_string(),
            primary_type: Some("Album".to_string()),
            secondary_types: vec![],
            first_release_date: Some("2005-05-11".to_string()),
            artist_credits: vec![],
        };

        let breakdown = DeterministicMatcher::score_candidate(
            &local,
            &candidate_track,
            None,
            Some(&candidate_rg),
        );

        assert!(
            breakdown.total_score >= 0.95,
            "Expected high confidence >= 0.95, got {}",
            breakdown.total_score
        );
        assert_eq!(breakdown.confidence_tier, ConfidenceTier::High);
    }

    #[test]
    fn test_integration_remastered_version_suffix_handling() {
        let local = LocalTrackMetadata {
            title: "Time (2011 Remastered Version)".to_string(),
            artist: Some("Pink Floyd".to_string()),
            album: Some("The Dark Side of the Moon (Deluxe Edition)".to_string()),
            album_artist: Some("Pink Floyd".to_string()),
            duration_ms: 425_000,
            track_number: Some(4),
            disc_number: Some(1),
            year: Some(1973),
            isrc: None,
            musicbrainz_track_id: None,
            musicbrainz_release_group_id: None,
        };

        let candidate_track = OnlineTrack {
            recording_mbid: "pf-rec-time".to_string(),
            release_mbid: None,
            release_group_mbid: None,
            position: Some(4),
            number: Some("4".to_string()),
            title: "Time".to_string(),
            duration_ms: Some(425_200),
            artist_credits: vec![OnlineArtistCredit {
                artist_mbid: "pf-art".to_string(),
                name: "Pink Floyd".to_string(),
                join_phrase: None,
            }],
            isrcs: vec![],
        };

        let candidate_rg = OnlineReleaseGroup {
            mbid: "pf-rg-dsotm".to_string(),
            title: "The Dark Side of the Moon".to_string(),
            primary_type: Some("Album".to_string()),
            secondary_types: vec![],
            first_release_date: Some("1973-03-01".to_string()),
            artist_credits: vec![],
        };

        let breakdown = DeterministicMatcher::score_candidate(
            &local,
            &candidate_track,
            None,
            Some(&candidate_rg),
        );

        assert!(
            breakdown.total_score >= 0.90,
            "Expected >= 0.90 score, got {}",
            breakdown.total_score
        );
        assert_eq!(breakdown.confidence_tier, ConfidenceTier::High);
    }

    #[test]
    fn test_integration_duration_differences_and_live_mismatch() {
        let local = LocalTrackMetadata {
            title: "Comfortably Numb".to_string(),
            artist: Some("Pink Floyd".to_string()),
            album: Some("The Wall".to_string()),
            album_artist: Some("Pink Floyd".to_string()),
            duration_ms: 382_000, // 6m22s Studio
            track_number: Some(6),
            disc_number: Some(2),
            year: Some(1979),
            isrc: None,
            musicbrainz_track_id: None,
            musicbrainz_release_group_id: None,
        };

        let studio_rg = OnlineReleaseGroup {
            mbid: "pf-rg-wall".to_string(),
            title: "The Wall".to_string(),
            primary_type: Some("Album".to_string()),
            secondary_types: vec![],
            first_release_date: Some("1979-11-30".to_string()),
            artist_credits: vec![],
        };

        let live_rg = OnlineReleaseGroup {
            mbid: "pf-rg-pulse".to_string(),
            title: "Pulse".to_string(),
            primary_type: Some("Album".to_string()),
            secondary_types: vec!["Live".to_string()],
            first_release_date: Some("1995-05-29".to_string()),
            artist_credits: vec![],
        };

        // Candidate 1: Exact Studio Track
        let studio_track = OnlineTrack {
            recording_mbid: "pf-rec-cn-studio".to_string(),
            release_mbid: None,
            release_group_mbid: None,
            position: Some(6),
            number: Some("6".to_string()),
            title: "Comfortably Numb".to_string(),
            duration_ms: Some(382_500),
            artist_credits: vec![OnlineArtistCredit {
                artist_mbid: "pf-art".to_string(),
                name: "Pink Floyd".to_string(),
                join_phrase: None,
            }],
            isrcs: vec![],
        };

        // Candidate 2: Pulse Live Version (8m45s = 525s -> Delta = 143s)
        let live_track = OnlineTrack {
            recording_mbid: "pf-rec-cn-live".to_string(),
            release_mbid: None,
            release_group_mbid: None,
            position: Some(12),
            number: Some("12".to_string()),
            title: "Comfortably Numb (Live at Earls Court)".to_string(),
            duration_ms: Some(525_000),
            artist_credits: vec![OnlineArtistCredit {
                artist_mbid: "pf-art".to_string(),
                name: "Pink Floyd".to_string(),
                join_phrase: None,
            }],
            isrcs: vec![],
        };

        let studio_breakdown =
            DeterministicMatcher::score_candidate(&local, &studio_track, None, Some(&studio_rg));
        let live_breakdown =
            DeterministicMatcher::score_candidate(&local, &live_track, None, Some(&live_rg));

        assert!(studio_breakdown.total_score >= 0.95);
        assert_eq!(studio_breakdown.confidence_tier, ConfidenceTier::High);

        // Live track duration penalty (> 10s delta -> 0.0 duration score)
        assert_eq!(live_breakdown.duration_score, 0.0);
        assert!(live_breakdown.total_score < studio_breakdown.total_score);
    }

    #[test]
    fn test_integration_ambiguous_artists() {
        // Two artists with same name: John Williams (Composer) vs John Williams (Guitarist)
        let local = LocalTrackMetadata {
            title: "Main Title (from Star Wars)".to_string(),
            artist: Some("John Williams".to_string()),
            album: Some("Star Wars: A New Hope".to_string()),
            album_artist: Some("John Williams".to_string()),
            duration_ms: 320_000,
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(1977),
            isrc: None,
            musicbrainz_track_id: None,
            musicbrainz_release_group_id: None,
        };

        let star_wars_rg = OnlineReleaseGroup {
            mbid: "jw-rg-starwars".to_string(),
            title: "Star Wars: A New Hope".to_string(),
            primary_type: Some("Soundtrack".to_string()),
            secondary_types: vec![],
            first_release_date: Some("1977-10-01".to_string()),
            artist_credits: vec![],
        };

        let guitar_rg = OnlineReleaseGroup {
            mbid: "jw-rg-guitar".to_string(),
            title: "The Spanish Guitar of John Williams".to_string(),
            primary_type: Some("Album".to_string()),
            secondary_types: vec![],
            first_release_date: Some("1970-01-01".to_string()),
            artist_credits: vec![],
        };

        let star_wars_track = OnlineTrack {
            recording_mbid: "jw-starwars".to_string(),
            release_mbid: None,
            release_group_mbid: None,
            position: Some(1),
            number: Some("1".to_string()),
            title: "Main Title".to_string(),
            duration_ms: Some(320_500),
            artist_credits: vec![OnlineArtistCredit {
                artist_mbid: "jw-composer-mbid".to_string(),
                name: "John Williams".to_string(),
                join_phrase: None,
            }],
            isrcs: vec![],
        };

        let guitar_track = OnlineTrack {
            recording_mbid: "jw-guitar".to_string(),
            release_mbid: None,
            release_group_mbid: None,
            position: Some(1),
            number: Some("1".to_string()),
            title: "Cavatina".to_string(),
            duration_ms: Some(210_000),
            artist_credits: vec![OnlineArtistCredit {
                artist_mbid: "jw-guitarist-mbid".to_string(),
                name: "John Williams".to_string(),
                join_phrase: None,
            }],
            isrcs: vec![],
        };

        let sw_breakdown = DeterministicMatcher::score_candidate(
            &local,
            &star_wars_track,
            None,
            Some(&star_wars_rg),
        );
        let guitar_breakdown =
            DeterministicMatcher::score_candidate(&local, &guitar_track, None, Some(&guitar_rg));

        assert!(sw_breakdown.total_score > guitar_breakdown.total_score);
        assert!(sw_breakdown.total_score >= 0.90);
        assert_eq!(sw_breakdown.confidence_tier, ConfidenceTier::High);
        assert!(guitar_breakdown.total_score < 0.65);
        assert_eq!(guitar_breakdown.confidence_tier, ConfidenceTier::Low);
    }

    // ========================================================================
    // Integration Tests: Mock HTTP Server & MusicBrainz Failure Cases
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
    async fn test_integration_mb_http_429_retry_after() {
        let (server_url, _handle) = start_mock_mb_server(
            429,
            r#"{"error": "Too Many Requests"}"#,
            vec![("Retry-After", "2")],
        )
        .await;

        let client = MusicBrainzClient::with_config(
            server_url,
            "Sonora/Test".to_string(),
            Duration::from_secs(2),
            RateLimiter::new(Duration::from_millis(0)),
        );

        let err = client.search_artists("Pink Floyd", 5).await.unwrap_err();
        match err {
            ProviderError::RateLimited { retry_after_secs } => {
                assert_eq!(retry_after_secs, Some(2));
            }
            other => panic!("Expected RateLimited error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_integration_mb_timeout_handling() {
        // Bind a listener that never responds
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let _handle = tokio::spawn(async move {
            if let Ok((mut _socket, _)) = listener.accept().await {
                // Sleep indefinitely without writing
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });

        let client = MusicBrainzClient::with_config(
            format!("http://{}", addr),
            "Sonora/Test".to_string(),
            Duration::from_millis(100), // Short timeout for test
            RateLimiter::new(Duration::from_millis(0)),
        );

        let err = client.search_artists("Pink Floyd", 5).await.unwrap_err();
        assert!(matches!(err, ProviderError::Timeout));
    }

    #[tokio::test]
    async fn test_integration_mb_offline_network_failure() {
        // Connect to a closed port
        let client = MusicBrainzClient::with_config(
            "http://127.0.0.1:59999".to_string(),
            "Sonora/Test".to_string(),
            Duration::from_millis(500),
            RateLimiter::new(Duration::from_millis(0)),
        );

        let err = client.search_artists("Pink Floyd", 5).await.unwrap_err();
        assert!(matches!(
            err,
            ProviderError::Offline | ProviderError::Network(_)
        ));
    }

    #[tokio::test]
    async fn test_integration_mb_malformed_json_response() {
        let (server_url, _handle) =
            start_mock_mb_server(200, r#"{"invalid_json": true, "corrupted": [}"#, vec![]).await;

        let client = MusicBrainzClient::with_config(
            server_url,
            "Sonora/Test".to_string(),
            Duration::from_secs(2),
            RateLimiter::new(Duration::from_millis(0)),
        );

        let err = client.search_artists("Pink Floyd", 5).await.unwrap_err();
        assert!(matches!(err, ProviderError::Parse(_)));
    }

    #[tokio::test]
    async fn test_integration_rate_limiter_throttles_rapid_requests() {
        let limiter = RateLimiter::new(Duration::from_millis(30));

        let start = Instant::now();
        for _ in 0..4 {
            limiter.acquire().await;
        }
        let elapsed = start.elapsed();

        // 4 requests with 30ms interval = >= 90ms elapsed
        assert!(
            elapsed >= Duration::from_millis(80),
            "Expected elapsed >= 80ms, got {:?}",
            elapsed
        );
    }
}
