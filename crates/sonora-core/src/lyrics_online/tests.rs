use crate::lyrics_online::lrclib::LrclibClient;
use crate::lyrics_online::models::{LyricsCandidateQuery, LyricsSyncType};
use crate::lyrics_online::traits::OnlineLyricsProvider;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_lrclib_synced_lyrics_lookup_and_ast_conversion() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;

            let response_body = r#"{
                "id": 1001,
                "name": "Karma Police",
                "artistName": "Radiohead",
                "albumName": "OK Computer",
                "duration": 264.0,
                "instrumental": false,
                "plainLyrics": "Karma police, arrest this man\nHe talks in maths",
                "syncedLyrics": "[00:15.20] Karma police, arrest this man\n[00:23.50] He talks in maths"
            }"#;

            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(http_resp.as_bytes()).await;
        }
    });

    let client = LrclibClient::with_options(
        &format!("http://127.0.0.1:{}", port),
        "SonoraTest/0.2",
        Duration::from_millis(0),
    );

    let query = LyricsCandidateQuery {
        track_name: "Karma Police".to_string(),
        artist_name: Some("Radiohead".to_string()),
        album_name: Some("OK Computer".to_string()),
        duration_seconds: Some(264.0),
    };

    let cand = client
        .get_lyrics(&query)
        .await
        .expect("LRCLIB get failed")
        .expect("Expected candidate");

    assert_eq!(cand.sync_type, LyricsSyncType::LineSynced);
    assert_eq!(cand.track_name, "Karma Police");
    assert_eq!(cand.artist_name, "Radiohead");
    assert!(cand.match_confidence >= 0.95);
    assert!((cand.duration_delta_seconds - 0.0).abs() < 0.001);

    // Convert into Sonora Universal LyricsDocument AST
    let doc =
        LrclibClient::parse_candidate_document(&cand).expect("Failed to parse LyricsDocument AST");

    assert_eq!(doc.lines.len(), 2);
    assert_eq!(doc.lines[0].start_time_ms, 15200);
    assert_eq!(doc.lines[0].text, "Karma police, arrest this man");
    assert_eq!(doc.lines[1].start_time_ms, 23500);
    assert_eq!(doc.lines[1].text, "He talks in maths");
}

#[tokio::test]
async fn test_lrclib_plain_lyrics_lookup() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;

            let response_body = r#"{
                "id": 1002,
                "name": "Subterranean Homesick Alien",
                "artistName": "Radiohead",
                "duration": 267.0,
                "instrumental": false,
                "plainLyrics": "The breath of the morning\nI keep forgetting",
                "syncedLyrics": null
            }"#;

            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(http_resp.as_bytes()).await;
        }
    });

    let client = LrclibClient::with_options(
        &format!("http://127.0.0.1:{}", port),
        "SonoraTest/0.2",
        Duration::from_millis(0),
    );

    let query = LyricsCandidateQuery {
        track_name: "Subterranean Homesick Alien".to_string(),
        artist_name: Some("Radiohead".to_string()),
        album_name: None,
        duration_seconds: Some(267.0),
    };

    let cand = client
        .get_lyrics(&query)
        .await
        .expect("LRCLIB get failed")
        .expect("Expected candidate");

    assert_eq!(cand.sync_type, LyricsSyncType::PlainText);
    let doc = LrclibClient::parse_candidate_document(&cand).unwrap();
    assert_eq!(doc.lines.len(), 2);
    assert_eq!(doc.lines[0].text, "The breath of the morning");
}

#[tokio::test]
async fn test_lrclib_instrumental_handling() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;

            let response_body = r#"{
                "id": 1003,
                "name": "Meeting in the Aisle",
                "artistName": "Radiohead",
                "duration": 190.0,
                "instrumental": true,
                "plainLyrics": null,
                "syncedLyrics": null
            }"#;

            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(http_resp.as_bytes()).await;
        }
    });

    let client = LrclibClient::with_options(
        &format!("http://127.0.0.1:{}", port),
        "SonoraTest/0.2",
        Duration::from_millis(0),
    );

    let query = LyricsCandidateQuery {
        track_name: "Meeting in the Aisle".to_string(),
        artist_name: Some("Radiohead".to_string()),
        album_name: None,
        duration_seconds: Some(190.0),
    };

    let cand = client.get_lyrics(&query).await.unwrap().unwrap();
    assert!(cand.is_instrumental);
    let doc = LrclibClient::parse_candidate_document(&cand).unwrap();
    assert!(doc.lines.is_empty());
}

#[tokio::test]
async fn test_lrclib_search_multi_candidate_ranking() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;

            let response_body = r#"[
                {
                    "id": 2001,
                    "name": "Creep (Live at Reading 1993)",
                    "artistName": "Radiohead",
                    "duration": 255.0,
                    "instrumental": false,
                    "plainLyrics": "When you were here before...",
                    "syncedLyrics": null
                },
                {
                    "id": 2002,
                    "name": "Creep",
                    "artistName": "Radiohead",
                    "albumName": "Pablo Honey",
                    "duration": 238.0,
                    "instrumental": false,
                    "plainLyrics": "When you were here before\nCouldn't look you in the eye",
                    "syncedLyrics": "[00:10.00] When you were here before\n[00:15.00] Couldn't look you in the eye"
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

    let client = LrclibClient::with_options(
        &format!("http://127.0.0.1:{}", port),
        "SonoraTest/0.2",
        Duration::from_millis(0),
    );

    let query = LyricsCandidateQuery {
        track_name: "Creep".to_string(),
        artist_name: Some("Radiohead".to_string()),
        album_name: Some("Pablo Honey".to_string()),
        duration_seconds: Some(238.0),
    };

    let candidates = client
        .search_lyrics(&query)
        .await
        .expect("LRCLIB search failed");

    assert_eq!(candidates.len(), 2);
    // Synced studio version with exact duration should rank higher than live plain version
    assert_eq!(candidates[0].candidate_id, "lrclib_2002");
    assert_eq!(candidates[0].sync_type, LyricsSyncType::LineSynced);
    assert!(candidates[0].match_confidence > candidates[1].match_confidence);
}

#[tokio::test]
async fn test_lrclib_http_429_retry_after() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;

            let http_resp =
                "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 3\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(http_resp.as_bytes()).await;
        }
    });

    let client = LrclibClient::with_options(
        &format!("http://127.0.0.1:{}", port),
        "SonoraTest/0.2",
        Duration::from_millis(0),
    );

    let query = LyricsCandidateQuery {
        track_name: "Karma Police".to_string(),
        artist_name: Some("Radiohead".to_string()),
        album_name: None,
        duration_seconds: None,
    };

    let res = client.get_lyrics(&query).await;
    assert!(res.is_err());
    assert!(matches!(
        res.unwrap_err(),
        crate::metadata::ProviderError::RateLimited {
            retry_after_secs: Some(3)
        }
    ));
}
