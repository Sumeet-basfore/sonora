use crate::artwork::coverartarchive::CoverArtArchiveClient;
use crate::artwork::fanart::FanartTvArtworkProvider;
use crate::artwork::models::{ArtworkKind, ArtworkSourceType};
use crate::artwork::pipeline::{
    ArtworkPipeline, FULL_ARTWORK_DIMENSION, MAX_IMAGE_DIMENSION, MAX_IMAGE_PAYLOAD_BYTES,
    THUMBNAIL_DIMENSION,
};
use crate::artwork::traits::ArtworkProvider;
use crate::artwork::wikidata::WikidataArtworkProvider;
use image::{ImageBuffer, ImageFormat, Rgb};
use std::io::Cursor;
use std::time::Duration;
use tokio::net::TcpListener;

fn create_test_image(width: u32, height: u32) -> Vec<u8> {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
        Rgb([(x % 255) as u8, (y % 255) as u8, 128])
    });
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("Failed to encode PNG");
    bytes
}

fn create_temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sonora_art_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_artwork_pipeline_valid_image() {
    let temp_dir = create_temp_dir();
    let pipeline = ArtworkPipeline::new(&temp_dir);

    let raw = create_test_image(400, 400);
    let asset = pipeline
        .process_and_cache("test_image_1", &raw)
        .expect("Valid image processing failed");

    assert_eq!(asset.width, 400);
    assert_eq!(asset.height, 400);
    assert!(std::path::Path::new(&asset.full_path).exists());
    assert!(std::path::Path::new(&asset.thumbnail_path).exists());

    // Second call should return existing cache without reprocessing
    let cached = pipeline.get_cached_asset("test_image_1");
    assert!(cached.is_some());
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_artwork_pipeline_large_image_downscaling() {
    let temp_dir = create_temp_dir();
    let pipeline = ArtworkPipeline::new(&temp_dir);

    // 2000x2000 image should be resized down to 1200x1200 max in full and 300x300 in thumb
    let raw = create_test_image(1600, 1600);
    let asset = pipeline
        .process_and_cache("large_image_1", &raw)
        .expect("Large image processing failed");

    assert_eq!(asset.width, FULL_ARTWORK_DIMENSION);
    assert_eq!(asset.height, FULL_ARTWORK_DIMENSION);

    let thumb_img = image::open(&asset.thumbnail_path).expect("Failed to open thumbnail");
    assert_eq!(thumb_img.width(), THUMBNAIL_DIMENSION);
    assert_eq!(thumb_img.height(), THUMBNAIL_DIMENSION);
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_artwork_pipeline_oversized_payload_rejection() {
    let temp_dir = create_temp_dir();
    let pipeline = ArtworkPipeline::new(&temp_dir);

    // Create a dummy byte slice exceeding 64 MB
    let oversized_bytes = vec![0u8; MAX_IMAGE_PAYLOAD_BYTES + 1024];
    let res = pipeline.process_and_cache("oversized_payload", &oversized_bytes);
    assert!(res.is_err());
    assert!(matches!(
        res.unwrap_err(),
        crate::metadata::ProviderError::Parse(_)
    ));
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_artwork_pipeline_malformed_image_rejection() {
    let temp_dir = create_temp_dir();
    let pipeline = ArtworkPipeline::new(&temp_dir);

    let garbage = b"NOT_A_VALID_IMAGE_FORMAT_RANDOM_GARBAGE";
    let res = pipeline.process_and_cache("malformed_img", garbage);
    assert!(res.is_err());
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_artwork_pipeline_oversized_dimension_rejection() {
    let temp_dir = create_temp_dir();
    let pipeline = ArtworkPipeline::new(&temp_dir);

    // Create a 1x1 PNG and mutate the IHDR dimensions to 9000x9000 (> 8192 MAX_IMAGE_DIMENSION)
    let mut png = create_test_image(1, 1);
    // PNG IHDR width is at offset 16..20, height at 20..24
    if png.len() >= 24 {
        let oversized: u32 = MAX_IMAGE_DIMENSION + 500;
        png[16..20].copy_from_slice(&oversized.to_be_bytes());
        png[20..24].copy_from_slice(&oversized.to_be_bytes());
    }

    let res = pipeline.process_and_cache("oversized_dim_img", &png);
    assert!(res.is_err());
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_caa_client_release_artwork() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;

            let response_body = r#"{
                "images": [
                    {
                        "id": "12345",
                        "image": "http://coverartarchive.org/release/76df3287/12345.jpg",
                        "thumbnails": {
                            "250": "http://coverartarchive.org/release/76df3287/12345-250.jpg",
                            "500": "http://coverartarchive.org/release/76df3287/12345-500.jpg",
                            "1200": "http://coverartarchive.org/release/76df3287/12345-1200.jpg"
                        },
                        "front": true,
                        "back": false,
                        "types": ["Front"],
                        "approved": true
                    },
                    {
                        "id": "67890",
                        "image": "http://coverartarchive.org/release/76df3287/67890.jpg",
                        "thumbnails": {
                            "500": "http://coverartarchive.org/release/76df3287/67890-500.jpg"
                        },
                        "front": false,
                        "back": true,
                        "types": ["Back"],
                        "approved": true
                    }
                ],
                "release": "http://musicbrainz.org/release/76df3287"
            }"#;

            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(http_resp.as_bytes()).await;
        }
    });

    let client = CoverArtArchiveClient::with_options(
        &format!("http://127.0.0.1:{}", port),
        "SonoraTest/0.2",
        Duration::from_millis(0),
    );

    let candidates = client
        .fetch_release_artwork("76df3287-6cda-33eb-8e0a-038825843c4a")
        .await
        .expect("CAA release artwork failed");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].kind, ArtworkKind::FrontCover);
    assert!(candidates[0].is_canonical);
    assert_eq!(
        candidates[0].preview_thumbnail_url,
        "http://coverartarchive.org/release/76df3287/12345-500.jpg"
    );

    assert_eq!(candidates[1].kind, ArtworkKind::BackCover);
    assert!(!candidates[1].is_canonical);
}

#[tokio::test]
async fn test_caa_client_release_group_artwork() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;

            let response_body = r#"{
                "images": [
                    {
                        "id": 99999,
                        "image": "http://coverartarchive.org/release-group/rg1/99999.jpg",
                        "thumbnails": {
                            "large": "http://coverartarchive.org/release-group/rg1/99999-500.jpg"
                        },
                        "front": true,
                        "back": false,
                        "types": ["Front"]
                    }
                ],
                "releaseGroup": "http://musicbrainz.org/release-group/rg1"
            }"#;

            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(http_resp.as_bytes()).await;
        }
    });

    let client = CoverArtArchiveClient::with_options(
        &format!("http://127.0.0.1:{}", port),
        "SonoraTest/0.2",
        Duration::from_millis(0),
    );

    let candidates = client
        .fetch_release_group_artwork("rg1")
        .await
        .expect("CAA release-group failed");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].kind, ArtworkKind::FrontCover);
    assert_eq!(
        candidates[0].source_type,
        ArtworkSourceType::CoverArtArchive {
            release_mbid: None,
            release_group_mbid: Some("rg1".to_string()),
        }
    );
}

#[tokio::test]
async fn test_wikidata_artist_portrait_provider() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;

            let response_body = r#"{
                "results": {
                    "bindings": [
                        {
                            "image": {
                                "type": "uri",
                                "value": "https://commons.wikimedia.org/wiki/Special:FilePath/Radiohead_2016.jpg"
                            }
                        }
                    ]
                }
            }"#;

            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(http_resp.as_bytes()).await;
        }
    });

    let provider = WikidataArtworkProvider::with_endpoint(&format!("http://127.0.0.1:{}", port));
    let candidates = provider
        .fetch_artist_artwork("a74b1b7f-71a5-4011-9441-d0b5e4122711", "Radiohead")
        .await
        .expect("Wikidata SPARQL failed");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].kind, ArtworkKind::ArtistPortrait);
    assert!(candidates[0].preview_thumbnail_url.contains("width=500"));
}

#[tokio::test]
async fn test_fanart_unconfigured_api_key_graceful_noop() {
    let provider = FanartTvArtworkProvider::new(None);
    let candidates = provider
        .fetch_artist_artwork("a74b1b7f-71a5-4011-9441-d0b5e4122711", "Radiohead")
        .await
        .expect("Unconfigured Fanart should gracefully return empty vec");

    assert!(candidates.is_empty());
}
