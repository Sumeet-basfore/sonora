//! Integration tests for Sonora v0.2 Storage, Artwork & Lyrics Providers, and Enrichment Router.

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, ImageFormat, Rgb};
    use sonora_core::artwork::{
        ArtworkKind, ArtworkPipeline, ArtworkQuery, CoverArtArchiveClient, FULL_ARTWORK_DIMENSION,
        THUMBNAIL_DIMENSION,
    };
    use sonora_core::enrichment::EnrichmentRouter;
    use sonora_core::lyrics_online::{LrclibClient, LyricsCandidateQuery, LyricsSyncType};
    use sonora_core::metadata::models::LocalTrackMetadata;
    use sonora_core::metadata::MusicBrainzClient;
    use sonora_library::Database;
    use std::io::Cursor;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::net::TcpListener;

    fn create_sample_png(width: u32, height: u32) -> Vec<u8> {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([(x % 255) as u8, (y % 255) as u8, 200])
        });
        let mut bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("Failed to encode sample PNG");
        bytes
    }

    fn create_temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sonora_test_enrichment_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_integration_router_metadata_and_stale_fallback() {
        let db = Arc::new(Database::in_memory().expect("In-memory DB"));

        // 1. Mock MusicBrainz server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            // First request succeeds
            if let Ok((mut stream, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;

                let response_body = r#"{
                "recordings": [
                    {
                        "id": "rec_ok_computer_1",
                        "title": "Airbag",
                        "length": 284000,
                        "artist-credit": [
                            {
                                "name": "Radiohead",
                                "artist": {
                                    "id": "art_radiohead_1",
                                    "name": "Radiohead",
                                    "sort-name": "Radiohead"
                                }
                            }
                        ],
                        "releases": [
                            {
                                "id": "rel_ok_1",
                                "title": "OK Computer",
                                "date": "1997-05-21",
                                "release-group": {
                                    "id": "rg_ok_1",
                                    "primary-type": "Album"
                                }
                            }
                        ]
                    }
                ]
            }"#;

                let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
                let _ = stream.write_all(http_resp.as_bytes()).await;
            }

            // Subsequent requests fail with 500 error to test stale cache fallback
            while let Ok((mut stream, _)) = listener.accept().await {
                use tokio::io::AsyncWriteExt;
                let http_resp = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n";
                let _ = stream.write_all(http_resp.as_bytes()).await;
            }
        });

        let mb_client = Arc::new(MusicBrainzClient::with_config(
            format!("http://127.0.0.1:{}", port),
            "SonoraTest/0.2".to_string(),
            Duration::from_secs(5),
            sonora_core::RateLimiter::new(Duration::from_millis(0)),
        ));

        let temp_dir = create_temp_dir();
        let pipeline = ArtworkPipeline::new(&temp_dir);
        let router = EnrichmentRouter::with_providers(
            db,
            mb_client,
            vec![],
            Arc::new(LrclibClient::new()),
            pipeline,
        );

        let local = LocalTrackMetadata {
            title: "Airbag".to_string(),
            artist: Some("Radiohead".to_string()),
            album: Some("OK Computer".to_string()),
            album_artist: None,
            track_number: Some(1),
            disc_number: Some(1),
            duration_ms: 284000,
            year: Some(1997),
            isrc: None,
            musicbrainz_track_id: None,
            musicbrainz_release_group_id: None,
        };

        // First lookup fetches from remote and caches
        let candidates = router
            .find_metadata_candidates(&local)
            .await
            .expect("First metadata search failed");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_track.title, "Airbag");
        assert!(candidates[0].score_breakdown.total_score >= 0.85);

        // Second lookup hits the cache directly (even though server is now returning 500)
        let cached_candidates = router
            .find_metadata_candidates(&local)
            .await
            .expect("Cached metadata search failed");

        assert_eq!(cached_candidates.len(), 1);
        assert_eq!(cached_candidates[0].candidate_track.title, "Airbag");
    }

    #[tokio::test]
    async fn test_integration_router_artwork_candidate_and_safe_caching() {
        let db = Arc::new(Database::in_memory().expect("In-memory DB"));

        // 1. Mock CAA server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // 2. Mock Image Hosting CDN Server
        let img_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let img_port = img_listener.local_addr().unwrap().port();

        let sample_image_data = create_sample_png(1500, 1500);
        let sample_image_bytes = sample_image_data.clone();

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = img_listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;

                let http_resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\n\r\n",
                    sample_image_bytes.len()
                );
                let _ = stream.write_all(http_resp.as_bytes()).await;
                let _ = stream.write_all(&sample_image_bytes).await;
            }
        });

        let img_url = format!("http://127.0.0.1:{}/cover.png", img_port);
        let img_url_clone = img_url.clone();

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;

                let response_body = format!(
                    r#"{{
                    "images": [
                        {{
                            "id": "991",
                            "image": "{}",
                            "thumbnails": {{
                                "500": "{}"
                            }},
                            "front": true,
                            "back": false,
                            "types": ["Front"]
                        }}
                    ],
                    "release": "http://musicbrainz.org/release/rel_test_1"
                }}"#,
                    img_url_clone, img_url_clone
                );

                let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
                let _ = stream.write_all(http_resp.as_bytes()).await;
            }
        });

        let caa_client = Arc::new(CoverArtArchiveClient::with_options(
            &format!("http://127.0.0.1:{}", port),
            "SonoraTest/0.2",
            Duration::from_millis(0),
        ));

        let temp_dir = create_temp_dir();
        let pipeline = ArtworkPipeline::new(&temp_dir);
        let router = EnrichmentRouter::with_providers(
            db,
            Arc::new(MusicBrainzClient::new()),
            vec![caa_client],
            Arc::new(LrclibClient::new()),
            pipeline,
        );

        let query = ArtworkQuery {
            release_mbid: Some("rel_test_1".to_string()),
            release_group_mbid: None,
            artist_mbid: None,
            artist_name: None,
            album_title: None,
        };

        let candidates = router
            .find_artwork_candidates(&query)
            .await
            .expect("Artwork candidates discovery failed");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].kind, ArtworkKind::FrontCover);
        assert_eq!(candidates[0].original_url, img_url);

        // Now test safe artwork download and WebP texture generation
        let cached_asset = router
            .download_and_cache_artwork(&img_url)
            .await
            .expect("Artwork download and caching failed");

        assert_eq!(cached_asset.width, FULL_ARTWORK_DIMENSION);
        assert_eq!(cached_asset.height, FULL_ARTWORK_DIMENSION);
        assert!(std::path::Path::new(&cached_asset.full_path).exists());
        assert!(std::path::Path::new(&cached_asset.thumbnail_path).exists());

        // Verify thumbnail is 300x300
        let thumb_img =
            image::open(&cached_asset.thumbnail_path).expect("Failed to open thumbnail");
        assert_eq!(thumb_img.width(), THUMBNAIL_DIMENSION);
        assert_eq!(thumb_img.height(), THUMBNAIL_DIMENSION);
    }

    #[tokio::test]
    async fn test_integration_router_lyrics_candidate_search_and_caching() {
        let db = Arc::new(Database::in_memory().expect("In-memory DB"));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;

                let response_body = r#"[
                {
                    "id": 5001,
                    "name": "Lucky",
                    "artistName": "Radiohead",
                    "albumName": "OK Computer",
                    "duration": 259.0,
                    "instrumental": false,
                    "plainLyrics": "I'm on a roll\nI'm on a roll this time",
                    "syncedLyrics": "[00:20.10] I'm on a roll\n[00:24.50] I'm on a roll this time"
                }
            ]"#;

                let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
                let _ = stream.write_all(http_resp.as_bytes()).await;
            }
        });

        let lrclib_client = Arc::new(LrclibClient::with_options(
            &format!("http://127.0.0.1:{}", port),
            "SonoraTest/0.2",
            Duration::from_millis(0),
        ));

        let temp_dir = create_temp_dir();
        let pipeline = ArtworkPipeline::new(&temp_dir);
        let router = EnrichmentRouter::with_providers(
            db,
            Arc::new(MusicBrainzClient::new()),
            vec![],
            lrclib_client,
            pipeline,
        );

        let query = LyricsCandidateQuery {
            track_name: "Lucky".to_string(),
            artist_name: Some("Radiohead".to_string()),
            album_name: Some("OK Computer".to_string()),
            duration_seconds: Some(259.0),
        };

        let candidates = router
            .find_lyrics_candidates(&query)
            .await
            .expect("Lyrics search failed");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].sync_type, LyricsSyncType::LineSynced);
        assert_eq!(candidates[0].track_name, "Lucky");

        // Verify parsed AST document
        let doc =
            LrclibClient::parse_candidate_document(&candidates[0]).expect("AST parsing failed");
        assert_eq!(doc.lines.len(), 2);
        assert_eq!(doc.lines[0].start_time_ms, 20100);
    }

    #[test]
    fn test_integration_cache_purge_expired_entries() {
        let db = Arc::new(Database::in_memory().expect("In-memory DB"));
        let router = EnrichmentRouter::new(db, None);

        // Insert one fresh item and one expired item (TTL = 0)
        let fp_fresh = "fresh_fingerprint";
        let fp_expired = "expired_fingerprint";

        router
            .cache()
            .set_metadata_candidates(fp_fresh, &[], Some(3600));
        router
            .cache()
            .set_metadata_candidates(fp_expired, &[], Some(0));

        // Fresh lookup before purge
        assert!(router
            .cache()
            .get_metadata_candidates(fp_fresh, false)
            .is_some());
        assert!(router
            .cache()
            .get_metadata_candidates(fp_expired, false)
            .is_none());

        // Purge expired entries
        let purged_count = router.cache().purge_expired();
        assert!(purged_count >= 1);

        // Stale lookup for expired item is now completely gone from DB
        assert!(router
            .cache()
            .get_metadata_candidates(fp_expired, true)
            .is_none());
        // Fresh item remains intact
        assert!(router
            .cache()
            .get_metadata_candidates(fp_fresh, false)
            .is_some());
    }
}
