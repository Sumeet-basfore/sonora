#[cfg(test)]
mod tests {
    use lofty::tag::{Accessor, Tag, TagExt, TagType};
    use sonora_audio::AudioRingBuffer;
    use sonora_common::{init_logging, LogConfig, PlaybackState, Result, SonoraError, TrackId};
    use sonora_core::{SonoraApp, SonoraConfig, SonoraEvent};
    use sonora_dsp::{
        db_to_linear, linear_to_db, BiquadCoefficients, BiquadFilter, ParametricEqualizer,
    };
    use sonora_library::{Database, LibraryRepository};
    use sonora_lyrics::{LyricsFormat, LyricsParser, PlainTextLyricsParser};
    use std::io::Write;
    use std::path::Path;
    use std::time::Duration;

    fn generate_pcm_wav(path: &Path, duration_secs: f32, freq_hz: f32) {
        let sample_rate: u32 = 44100;
        let channels: u16 = 2;
        let bits_per_sample: u16 = 16;
        let num_samples = (duration_secs * sample_rate as f32) as u32;
        let data_len = num_samples * channels as u32 * (bits_per_sample as u32 / 8);
        let file_len = 36 + data_len;

        let mut file = std::fs::File::create(path).expect("Failed to create WAV file");
        file.write_all(b"RIFF").unwrap();
        file.write_all(&file_len.to_le_bytes()).unwrap();
        file.write_all(b"WAVE").unwrap();
        file.write_all(b"fmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap(); // Subchunk1Size (16 for PCM)
        file.write_all(&1u16.to_le_bytes()).unwrap(); // AudioFormat (1 = PCM)
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
        file.write_all(&byte_rate.to_le_bytes()).unwrap();
        let block_align = channels * (bits_per_sample / 8);
        file.write_all(&block_align.to_le_bytes()).unwrap();
        file.write_all(&bits_per_sample.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();

        for i in 0..num_samples {
            let t = (i as f32) / (sample_rate as f32);
            let sample = (2.0 * std::f32::consts::PI * freq_hz * t).sin();
            let val = (sample * 16000.0) as i16;
            file.write_all(&val.to_le_bytes()).unwrap();
            file.write_all(&val.to_le_bytes()).unwrap();
        }
        file.flush().unwrap();
    }

    #[test]
    fn test_error_handling_and_propagation() {
        let err = SonoraError::Audio("Output device disconnected".to_string());
        assert_eq!(
            err.to_string(),
            "Audio subsystem error: Output device disconnected"
        );

        let res: Result<()> = Err(SonoraError::Database("Lock busy".to_string()));
        assert!(res.is_err());
    }

    #[test]
    fn test_logging_initialization() {
        init_logging(&LogConfig::default());
        init_logging(&LogConfig::default());
        tracing::info!("Logging initialized successfully in test suite");
    }

    #[test]
    fn test_sqlite_fts5_indexing_and_search() -> Result<()> {
        let db = Database::in_memory()?;
        let repo = LibraryRepository::new(&db);

        let artist_id = repo.upsert_artist("Daft Punk")?;
        let album_id = repo.upsert_album("Random Access Memories", Some(artist_id), Some(2013))?;

        let track_id = repo.insert_track(
            "/music/daft_punk/get_lucky.flac",
            "sha256:abc123mockhash",
            45000000,
            1690000000,
            "Get Lucky",
            Some(artist_id),
            Some(album_id),
            Some(8),
            1,
            248000,
            44100,
            Some(16),
            2,
            Some(920),
            "flac",
            Some("Electronic"),
        )?;

        assert_eq!(track_id, TrackId(1));

        // Test FTS5 trigram search
        let results = repo.search("Lucky", 10)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Get Lucky");
        assert_eq!(results[0].artist_name.as_deref(), Some("Daft Punk"));
        assert_eq!(
            results[0].album_title.as_deref(),
            Some("Random Access Memories")
        );

        Ok(())
    }

    #[test]
    fn test_dsp_biquad_and_equalizer() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-5);
        assert!((db_to_linear(6.0) - 1.99526).abs() < 1e-4);
        assert!((linear_to_db(1.0) - 0.0).abs() < 1e-5);

        let mut filter = BiquadFilter::new(BiquadCoefficients::identity());
        let sample = 0.75f32;
        assert_eq!(filter.process_sample(sample), sample);

        let mut eq = ParametricEqualizer::new(48000.0);
        eq.set_band(0, 3.0);
        let out = eq.process_sample(0.5f32);
        assert!(out.is_finite());
    }

    #[test]
    fn test_lockfree_ring_buffer() {
        let (mut prod, mut cons) = AudioRingBuffer::create(1024);

        for i in 0..128 {
            prod.push(i as f32).expect("Push should succeed");
        }

        for i in 0..128 {
            let sample = cons.pop().expect("Pop should succeed");
            assert_eq!(sample, i as f32);
        }

        assert!(cons.pop().is_err());
    }

    #[test]
    fn test_lyrics_ast_and_parser() -> Result<()> {
        let parser = PlainTextLyricsParser;
        assert_eq!(parser.format(), LyricsFormat::Plain);

        let doc = parser.parse("First line\nSecond line")?;
        assert_eq!(doc.lines.len(), 2);
        assert_eq!(doc.lines[0].text, "First line");
        assert_eq!(doc.lines[1].text, "Second line");
        Ok(())
    }

    #[tokio::test]
    async fn test_core_app_lifecycle_and_events() -> Result<()> {
        let config = SonoraConfig::default();
        let app = SonoraApp::in_memory(config)?;
        let mut rx = app.subscribe_events();

        app.broadcast_event(SonoraEvent::PlaybackStateChanged(PlaybackState::Playing));

        let received = rx
            .recv()
            .await
            .map_err(|e| SonoraError::Internal(e.to_string()))?;
        match received {
            SonoraEvent::PlaybackStateChanged(state) => assert_eq!(state, PlaybackState::Playing),
            _ => panic!("Unexpected event received"),
        }

        Ok(())
    }

    #[test]
    fn test_scan_prunes_deleted_files() -> Result<()> {
        let test_dir = std::env::temp_dir().join(format!(
            "sonora_test_prune_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&test_dir).expect("Failed to create test dir");
        let keep_path = test_dir.join("keep.wav");
        let gone_path = test_dir.join("gone.wav");
        generate_pcm_wav(&keep_path, 0.5, 440.0);
        generate_pcm_wav(&gone_path, 0.5, 880.0);

        let app = SonoraApp::in_memory(SonoraConfig::default())?;
        let stats = app.scan_directory(&test_dir)?;
        assert_eq!(stats.indexed_tracks, 2);
        assert_eq!(
            app.search("keep", 10)?.len() + app.search("gone", 10)?.len(),
            2
        );

        // Delete one file and rescan: the ghost row must disappear.
        std::fs::remove_file(&gone_path).unwrap();
        let rescan = app.scan_directory(&test_dir)?;
        assert_eq!(rescan.pruned_missing, 1);
        assert!(app.search("gone", 10)?.is_empty());
        assert_eq!(app.search("keep", 10)?.len(), 1);

        let _ = std::fs::remove_dir_all(&test_dir);
        Ok(())
    }

    #[test]
    fn test_scan_survives_symlink_cycle() -> Result<()> {
        let test_dir = std::env::temp_dir().join(format!(
            "sonora_test_symlink_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sub_dir = test_dir.join("sub");
        std::fs::create_dir_all(&sub_dir).expect("Failed to create test dir");
        generate_pcm_wav(&sub_dir.join("tone.wav"), 0.5, 440.0);
        // Symlink loop: sub/loop -> test_dir (would recurse forever unguarded).
        #[cfg(unix)]
        std::os::unix::fs::symlink(&test_dir, sub_dir.join("loop")).unwrap();

        let app = SonoraApp::in_memory(SonoraConfig::default())?;
        let stats = app.scan_directory(&test_dir)?;
        assert_eq!(stats.indexed_tracks, 1);
        assert_eq!(app.search("tone", 10)?.len(), 1);

        let _ = std::fs::remove_dir_all(&test_dir);
        Ok(())
    }

    #[test]
    fn test_end_to_end_user_flow() -> Result<()> {
        // 1. Create a temporary directory with test audio files
        let test_dir = std::env::temp_dir().join(format!(
            "sonora_test_flow_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&test_dir).expect("Failed to create test dir");

        let wav1_path = test_dir.join("get_lucky.wav");
        let wav2_path = test_dir.join("around_the_world.wav");

        generate_pcm_wav(&wav1_path, 1.5, 440.0);
        generate_pcm_wav(&wav2_path, 1.0, 880.0);

        // Tag with Lofty if supported
        let mut tag1 = Tag::new(TagType::Id3v2);
        tag1.set_title("Get Lucky".to_string());
        tag1.set_artist("Daft Punk".to_string());
        tag1.set_album("Random Access Memories".to_string());
        tag1.set_genre("Disco".to_string());
        use lofty::config::WriteOptions;
        let _ = tag1.save_to_path(&wav1_path, WriteOptions::default());

        let mut tag2 = Tag::new(TagType::Id3v2);
        tag2.set_title("Around the World".to_string());
        tag2.set_artist("Daft Punk".to_string());
        tag2.set_album("Homework".to_string());
        tag2.set_genre("Electronic".to_string());
        let _ = tag2.save_to_path(&wav2_path, WriteOptions::default());

        // 2. Initialize SonoraApp with in-memory database and virtual audio output
        let config = SonoraConfig::default();
        let app = SonoraApp::in_memory(config)?;

        // 3. Scan user-provided music directory
        let stats = app.scan_directory(&test_dir)?;
        assert_eq!(stats.scanned_files, 2);
        assert_eq!(stats.indexed_tracks, 2);
        assert_eq!(stats.errors, 0);

        // Verify mtime skip on second scan
        let stats_second = app.scan_directory(&test_dir)?;
        assert_eq!(stats_second.scanned_files, 2);
        assert_eq!(stats_second.skipped_unmodified, 2);
        assert_eq!(stats_second.indexed_tracks, 0);
        assert_eq!(stats_second.pruned_missing, 0);

        // 4. Search library
        let search_results = app.search("Lucky", 10)?;
        assert_eq!(search_results.len(), 1);
        let lucky_track = &search_results[0];
        assert!(lucky_track.title.contains("Lucky") || lucky_track.title.contains("get_lucky"));

        let daft_results = app.search("Daft", 10)?;
        assert!(!daft_results.is_empty());

        // 5. Open track and play
        let lucky_id = lucky_track.track_id;
        app.play_track_id(lucky_id)?;

        // Allow background audio thread to initialize and begin decoding
        std::thread::sleep(Duration::from_millis(150));

        let status = app.status();
        assert_eq!(status.state, PlaybackState::Playing);
        assert!(status.current_track.is_some());

        // 6. Test Pause and Resume
        app.pause()?;
        std::thread::sleep(Duration::from_millis(50));
        let status_paused = app.status();
        assert_eq!(status_paused.state, PlaybackState::Paused);

        app.resume()?;
        std::thread::sleep(Duration::from_millis(50));
        let status_resumed = app.status();
        assert_eq!(status_resumed.state, PlaybackState::Playing);

        // 7. Test Seek
        app.seek(500)?;
        std::thread::sleep(Duration::from_millis(50));
        let status_seek = app.status();
        assert!(status_seek.position_ms >= 400);

        // 8. Test Volume
        app.set_volume(0.65)?;
        let status_vol = app.status();
        assert!((status_vol.volume - 0.65).abs() < 1e-4);

        // 9. Basic Playback Queue
        let other_track_res = app.search("around", 10)?;
        if !other_track_res.is_empty() {
            let other_id = other_track_res[0].track_id;
            app.enqueue_track(other_id)?;
            assert_eq!(app.status().queue_length, 2);

            // Advance queue
            app.queue_next()?;
            std::thread::sleep(Duration::from_millis(100));
            let status_next = app.status();
            assert_eq!(status_next.state, PlaybackState::Playing);

            // Return to previous
            app.queue_previous()?;
            std::thread::sleep(Duration::from_millis(100));
        }

        // 10. Stop
        app.stop()?;
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(app.status().state, PlaybackState::Stopped);

        // Cleanup
        let _ = std::fs::remove_dir_all(&test_dir);

        Ok(())
    }

    #[test]
    fn test_fft_spectrum_pipeline() {
        use sonora_dsp::{SpectrumAnalyzer, DEFAULT_FFT_SIZE, DEFAULT_SPECTRUM_BANDS};
        let mut analyzer = SpectrumAnalyzer::new(DEFAULT_FFT_SIZE, DEFAULT_SPECTRUM_BANDS, 48000.0);

        // 1. Silence test
        let silence = vec![0.0f32; DEFAULT_FFT_SIZE];
        let spectrum = analyzer.compute_spectrum(&silence);
        assert_eq!(spectrum.len(), DEFAULT_SPECTRUM_BANDS);
        for &val in &spectrum {
            assert!(
                (0.0..=0.05).contains(&val),
                "Silence should yield near-zero dB, got {val}"
            );
        }

        // 2. 440 Hz Sine wave (Concert A) test
        let mut sine = vec![0.0f32; DEFAULT_FFT_SIZE];
        for (i, sample) in sine.iter_mut().enumerate() {
            *sample = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin();
        }
        let spectrum_sine = analyzer.compute_spectrum(&sine);
        assert_eq!(spectrum_sine.len(), DEFAULT_SPECTRUM_BANDS);
        let max_val = spectrum_sine.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            max_val > 0.6,
            "Expected high peak for 440Hz sine wave, got {max_val}"
        );
    }

    #[test]
    fn test_search_punctuation_and_short_queries() -> Result<()> {
        let db = Database::in_memory()?;
        let repo = LibraryRepository::new(&db);

        let artist_id = repo.upsert_artist("AC/DC")?;
        let album_id = repo.upsert_album("Back In Black", Some(artist_id), Some(1980))?;

        repo.insert_track(
            "/music/acdc/hells_bells.mp3",
            "hash1",
            1000,
            100,
            "Hells Bells",
            Some(artist_id),
            Some(album_id),
            Some(1),
            1,
            312000,
            44100,
            Some(16),
            2,
            Some(320),
            "mp3",
            Some("Rock"),
        )?;

        // 1. Search with punctuation like slash, hyphen, quotes
        let r1 = repo.search("AC/DC", 10)?;
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].artist_name.as_deref(), Some("AC/DC"));

        // 2. Short queries < 3 characters (e.g. "AC", "He")
        let r2 = repo.search("AC", 10)?;
        assert_eq!(r2.len(), 1);

        let r3 = repo.search("He", 10)?;
        assert_eq!(r3.len(), 1);

        // 3. Quotes & brackets
        let r4 = repo.search("\"Hells\"", 10)?;
        assert_eq!(r4.len(), 1);

        // 4. Empty query
        let r5 = repo.search("   ", 10)?;
        assert!(r5.is_empty());

        Ok(())
    }

    #[test]
    fn test_duplicate_albums_and_artists() -> Result<()> {
        let db = Database::in_memory()?;
        let repo = LibraryRepository::new(&db);

        // 1. Two distinct artists with an album of the same title
        let art1 = repo.upsert_artist("Artist One")?;
        let art2 = repo.upsert_artist("Artist Two")?;

        let alb1 = repo.upsert_album("Greatest Hits", Some(art1), Some(2000))?;
        let alb2 = repo.upsert_album("Greatest Hits", Some(art2), Some(2010))?;
        assert_ne!(
            alb1, alb2,
            "Distinct artists must have distinct album records for same title"
        );

        // 2. Duplicate insertion with same artist returns same ID
        let alb1_dup = repo.upsert_album("Greatest Hits", Some(art1), Some(2000))?;
        assert_eq!(alb1, alb1_dup);

        // 3. Album with NULL artist
        let alb_null1 = repo.upsert_album("Unknown Album", None, None)?;
        let alb_null2 = repo.upsert_album("Unknown Album", None, None)?;
        assert_eq!(
            alb_null1, alb_null2,
            "Duplicate null-artist albums should resolve to single album ID"
        );

        Ok(())
    }

    #[test]
    fn test_100_plus_tracks_library() -> Result<()> {
        let db = Database::in_memory()?;
        let repo = LibraryRepository::new(&db);

        let artist_id = repo.upsert_artist("Orchestra")?;
        let album_id = repo.upsert_album("Complete Symphonies", Some(artist_id), Some(2024))?;

        // Insert 120 tracks with long titles
        for i in 1..=120 {
            let title = format!("Symphony No. {} in D Minor, Op. {}, Allegro con brio ed appassionato molto vivace e maestoso", (i % 9) + 1, i * 3);
            let path = format!("/music/symphonies/track_{:03}.flac", i);
            repo.insert_track(
                &path,
                &format!("hash_{i}"),
                50000000,
                1700000000 + i,
                &title,
                Some(artist_id),
                Some(album_id),
                Some(i as i32),
                1,
                600000,
                96000,
                Some(24),
                2,
                Some(2400),
                "flac",
                Some("Classical"),
            )?;
        }

        let (tracks, albums, artists) = repo.get_library_counts()?;
        assert_eq!(tracks, 120);
        assert_eq!(albums, 1);
        assert_eq!(artists, 1);

        let album_tracks = repo.get_album_tracks(album_id)?;
        assert_eq!(album_tracks.len(), 120);
        assert_eq!(album_tracks[0].track_number, Some(1));
        assert_eq!(album_tracks[119].track_number, Some(120));

        let all_tracks = repo.get_all_tracks(50)?;
        assert_eq!(all_tracks.len(), 50);

        Ok(())
    }

    #[test]
    fn test_missing_and_corrupt_files() -> Result<()> {
        let config = SonoraConfig::default();
        let app = SonoraApp::in_memory(config)?;

        // 1. Missing file
        let res = app.play_file("/path/does/not/exist/missing_audio.flac");
        assert!(res.is_err());

        // 2. Corrupted file (zero bytes or garbage header)
        let temp_dir = std::env::temp_dir().join("sonora_corrupt_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let corrupt_path = temp_dir.join("corrupt.flac");
        std::fs::write(&corrupt_path, b"NOT_A_VALID_FLAC_HEADER_JUST_GARBAGE_BYTES").unwrap();

        let res_corrupt = app.play_file(&corrupt_path);
        // play_file succeeds in enqueuing & dispatching command; worker loop catches decode error gracefully
        assert!(res_corrupt.is_ok());

        std::thread::sleep(Duration::from_millis(100));
        let status = app.status();
        assert_eq!(status.state, PlaybackState::Stopped);

        let _ = std::fs::remove_dir_all(&temp_dir);
        Ok(())
    }

    #[test]
    fn test_artwork_cache_and_thumbnails() {
        use image::{ImageBuffer, ImageFormat, Rgb};
        use sonora_library::ArtworkCache;
        use std::io::Cursor;

        let cache = ArtworkCache::new();

        // Generate synthetic test image (512x512 gradient)
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(512, 512, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let mut jpeg_bytes = Cursor::new(Vec::new());
        img.write_to(&mut jpeg_bytes, ImageFormat::Jpeg).unwrap();
        let raw_jpeg = jpeg_bytes.into_inner();

        let temp_dir = std::env::temp_dir().join("sonora_art_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let cover_path = temp_dir.join("cover.jpg");
        std::fs::write(&cover_path, &raw_jpeg).unwrap();

        // Audio file in same directory
        let audio_path = temp_dir.join("song.mp3");
        std::fs::write(&audio_path, b"mock audio content").unwrap();

        // Fetch thumbnail
        let thumb_art = cache.get_or_load_track_artwork(42, &audio_path, true);
        assert!(thumb_art.is_some());
        let thumb_url = thumb_art.unwrap();
        assert!(thumb_url.starts_with("data:image/jpeg;base64,"));

        // Fetch full artwork
        let full_art = cache.get_or_load_track_artwork(42, &audio_path, false);
        assert!(full_art.is_some());
        let full_url = full_art.unwrap();
        assert!(full_url.starts_with("data:image/jpeg;base64,"));

        // Verify thumbnail is substantially smaller than full resolution
        assert!(
            thumb_url.len() < full_url.len(),
            "Thumbnail size ({}) should be smaller than full ({})",
            thumb_url.len(),
            full_url.len()
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_lyrics_lrc_parsing_and_timing() {
        use sonora_lyrics::parser::{LrcLyricsParser, LyricsParser};
        let parser = LrcLyricsParser;
        let lrc = r#"
[ti:Instant Crush]
[ar:Daft Punk]
[al:Random Access Memories]
[offset:+250]
[00:10.50]I didn't want to be the one to forget
[00:15.00]I thought of something, something I want to say
[00:20.00]It wouldn't hurt to think about it
"#;
        let doc = parser.parse(lrc).expect("parse LRC");
        assert_eq!(doc.title.as_deref(), Some("Instant Crush"));
        assert_eq!(doc.artist.as_deref(), Some("Daft Punk"));
        assert_eq!(doc.album.as_deref(), Some("Random Access Memories"));
        assert_eq!(doc.offset_ms, 250);
        assert_eq!(doc.lines.len(), 3);
        assert_eq!(doc.lines[0].start_time_ms, 10750); // 10500 + 250
        assert_eq!(doc.lines[0].text, "I didn't want to be the one to forget");
        assert_eq!(doc.lines[1].start_time_ms, 15250); // 15000 + 250
        assert_eq!(doc.lines[0].end_time_ms, Some(15250));
    }

    #[test]
    fn test_lyrics_sqlite_caching_and_offset_persistence() -> Result<()> {
        use sonora_lyrics::model::{LyricLine, LyricsDocument, LyricsFormat};

        let db = Database::in_memory()?;
        let repo = LibraryRepository::new(&db);

        let doc = LyricsDocument {
            title: Some("Harder, Better, Faster, Stronger".to_string()),
            artist: Some("Daft Punk".to_string()),
            album: Some("Discovery".to_string()),
            offset_ms: 0,
            format: LyricsFormat::Lrc,
            lines: vec![
                LyricLine {
                    start_time_ms: 1000,
                    end_time_ms: Some(3000),
                    text: "Work it make it".to_string(),
                    syllables: Vec::new(),
                },
                LyricLine {
                    start_time_ms: 3000,
                    end_time_ms: Some(5000),
                    text: "Do it makes us".to_string(),
                    syllables: Vec::new(),
                },
            ],
        };

        // Insert a real track so foreign key is satisfied
        let tid = repo.insert_track(
            "/music/daft_punk/hbfs.flac",
            "sha256:mockhbfs",
            10000000,
            1690000000,
            "Harder, Better, Faster, Stronger",
            None,
            None,
            Some(1),
            1,
            224000,
            44100,
            Some(16),
            2,
            Some(900),
            "flac",
            None,
        )?;

        // 1. Save to SQLite cache
        repo.save_cached_lyrics(
            Some(tid.0),
            Some("/music/daft_punk/hbfs.flac"),
            "Harder, Better, Faster, Stronger",
            Some("Daft Punk"),
            &doc,
            "test_provider",
        )?;

        // 2. Query by track_id
        let cached_by_id = repo.get_cached_lyrics(
            Some(tid.0),
            None,
            "Harder, Better, Faster, Stronger",
            Some("Daft Punk"),
        )?;
        assert!(cached_by_id.is_some());
        let hit = cached_by_id.unwrap();
        assert_eq!(hit.lines.len(), 2);
        assert_eq!(hit.lines[0].text, "Work it make it");
        assert_eq!(hit.offset_ms, 0);

        // 3. Query by file_path
        let cached_by_path = repo.get_cached_lyrics(
            None,
            Some("/music/daft_punk/hbfs.flac"),
            "Harder, Better, Faster, Stronger",
            None,
        )?;
        assert!(cached_by_path.is_some());

        // 4. Query by title & artist
        let cached_by_title = repo.get_cached_lyrics(
            None,
            None,
            "Harder, Better, Faster, Stronger",
            Some("Daft Punk"),
        )?;
        assert!(cached_by_title.is_some());

        // 5. Update manual timing offset
        repo.update_lyrics_offset(Some(tid.0), None, 300)?;
        let updated = repo
            .get_cached_lyrics(
                Some(tid.0),
                None,
                "Harder, Better, Faster, Stronger",
                Some("Daft Punk"),
            )?
            .unwrap();
        assert_eq!(updated.offset_ms, 300);

        Ok(())
    }

    #[test]
    fn test_lyrics_sidecar_resolution_and_fallback() {
        use sonora_lyrics::provider::{
            CascadingLyricsResolver, LocalSidecarLyricsProvider, TrackLyricsQuery,
        };
        use std::sync::Arc;

        let temp_dir = std::env::temp_dir().join("sonora_sidecar_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let song_file = temp_dir.join("track_01.mp3");
        std::fs::write(&song_file, b"dummy audio").unwrap();

        let lrc_file = temp_dir.join("track_01.lrc");
        std::fs::write(
            &lrc_file,
            "[00:05.00]Sidecar line 1\n[00:10.00]Sidecar line 2\n",
        )
        .unwrap();

        let resolver = CascadingLyricsResolver::new(vec![Arc::new(LocalSidecarLyricsProvider)]);

        let query = TrackLyricsQuery {
            title: "Track 01".to_string(),
            artist: Some("Artist".to_string()),
            album: None,
            duration_ms: Some(60000),
            file_path: Some(song_file),
            track_id: None,
        };

        let result = resolver.resolve(&query).expect("resolve sidecar");
        assert!(result.is_some());
        let (doc, provider_name) = result.unwrap();
        assert_eq!(provider_name, "local_lrc");
        assert_eq!(doc.lines.len(), 2);
        assert_eq!(doc.lines[0].text, "Sidecar line 1");
        assert_eq!(doc.lines[0].start_time_ms, 5000);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // ------------------------------------------------------------------
    // Event-driven service boundaries + plugin foundation (core side)
    // ------------------------------------------------------------------

    fn drain_events(rx: &mut tokio::sync::broadcast::Receiver<SonoraEvent>) -> Vec<SonoraEvent> {
        let mut out = Vec::new();
        while let Ok(e) = rx.try_recv() {
            out.push(e);
        }
        out
    }

    fn has_playing(events: &[SonoraEvent]) -> bool {
        events
            .iter()
            .any(|e| matches!(e, SonoraEvent::PlaybackStateChanged(PlaybackState::Playing)))
    }

    fn test_answer() -> (String, usize) {
        let json = "{\"format\":\"lrc\",\"content\":\"[00:01.00] Core cascade line\\n\"}";
        let len = json.len();
        let esc = json.replace('\\', "\\\\").replace('"', "\\\"");
        (esc, len)
    }

    fn test_lyrics_wat() -> String {
        let (esc, len) = test_answer();
        format!(
            r#"(module
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 4096))
  (func $alloc (export "alloc") (param $size i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap))
    (global.set $heap (i32.add (global.get $heap) (local.get $size)))
    (local.get $ptr))
  (data (i32.const 64) "{esc}")
  (global $res_ptr (mut i32) (i32.const 0))
  (global $res_len (mut i32) (i32.const 0))
  (func (export "plugin_api_version") (result i32) (i32.const 1))
  (func (export "plugin_init") (result i32) (i32.const 0))
  (func (export "lyrics_fetch") (param $p i32) (param $l i32) (result i32)
    (global.set $res_ptr (i32.const 64))
    (global.set $res_len (i32.const {len}))
    (i32.const 1))
  (func (export "lyrics_result_ptr") (result i32) (global.get $res_ptr))
  (func (export "lyrics_result_len") (result i32) (global.get $res_len))
)"#
        )
    }

    fn test_manifest(id: &str) -> sonora_plugin::PluginManifest {
        sonora_plugin::PluginManifest::from_json(&format!(
            r#"{{"id":"{id}","name":"Core Test Plugin","version":"0.1.0","author":"Tests","description":"Core integration plugin.","api_version":1,"capabilities":["lyrics:provider"],"entry":"plugin.wasm","allowed_domains":[]}}"#
        ))
        .expect("test manifest")
    }

    #[test]
    fn test_service_boundaries_emit_events() -> Result<()> {
        let dir = std::env::temp_dir().join("sonora_events_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test dir");
        let wav = dir.join("tone.wav");
        generate_pcm_wav(&wav, 1.0, 440.0);

        let app = SonoraApp::in_memory(SonoraConfig::default())?;
        let mut rx = app.subscribe_events();

        app.play_file(&wav)?;
        app.set_volume(0.5)?;
        app.pause()?;

        let events = drain_events(&mut rx);
        assert!(has_playing(&events), "missing Playing event: {events:?}");
        assert!(
            events.iter().any(|e| matches!(
                e,
                SonoraEvent::QueueChanged {
                    queue_length: 1,
                    ..
                }
            )),
            "missing QueueChanged event: {events:?}"
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                SonoraEvent::VolumeChanged { volume, .. } if (*volume - 0.5).abs() < 1e-4
            )),
            "missing VolumeChanged event: {events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SonoraEvent::PlaybackStateChanged(PlaybackState::Paused))),
            "missing Paused event: {events:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn test_command_plane_with_plugin_lifecycle() -> Result<()> {
        use sonora_core::SonoraCommand;

        let app = SonoraApp::in_memory(SonoraConfig::default())?;
        let mut rx = app.subscribe_events();

        let wasm = wat::parse_str(test_lyrics_wat()).expect("test wat");
        let id = "org.sonora.test.cmd";
        app.handle_command(SonoraCommand::PluginRegister {
            manifest: Box::new(test_manifest(id)),
            dir: std::path::PathBuf::from("core-test"),
        })?;
        app.handle_command(SonoraCommand::PluginLoadBytes {
            id: id.to_string(),
            wasm,
        })?;
        app.handle_command(SonoraCommand::PluginStart { id: id.to_string() })?;

        assert_eq!(
            app.plugin_host().state(id),
            Some(sonora_plugin::PluginState::Running)
        );
        let events = drain_events(&mut rx);
        assert!(
            events.iter().any(|e| matches!(
                e,
                SonoraEvent::PluginStateChanged { plugin_id, to, .. }
                if plugin_id == id && to == "running"
            )),
            "missing PluginStateChanged(running): {events:?}"
        );

        // Mutations through the command plane still work alongside plugins.
        app.handle_command(SonoraCommand::SetVolume(0.3))?;
        assert!((app.volume() - 0.3).abs() < 1e-4);

        app.handle_command(SonoraCommand::PluginStop { id: id.to_string() })?;
        app.handle_command(SonoraCommand::PluginUnload { id: id.to_string() })?;
        assert_eq!(
            app.plugin_host().state(id),
            Some(sonora_plugin::PluginState::Unloaded)
        );
        Ok(())
    }

    #[test]
    fn test_plugin_lyrics_cascade_via_app() -> Result<()> {
        use sonora_lyrics::provider::TrackLyricsQuery;

        let app = SonoraApp::in_memory(SonoraConfig::default())?;
        let mut rx = app.subscribe_events();

        let wasm = wat::parse_str(test_lyrics_wat()).expect("test wat");
        let id = "org.sonora.test.cascade";
        app.register_plugin(test_manifest(id), std::path::PathBuf::from("core-test"))?;
        app.plugin_host()
            .load_bytes(id, &wasm)
            .map_err(|e| SonoraError::Internal(format!("plugin load failed: {e}")))?;
        app.plugin_host()
            .start(id)
            .map_err(|e| SonoraError::Internal(format!("plugin start failed: {e}")))?;
        app.forward_plugin_events();

        // Gibberish title: built-ins (incl. LRCLIB) miss, the WASM provider hits.
        let query = TrackLyricsQuery {
            title: "Xyzzy Plugh Qqqq Nonexistent".to_string(),
            artist: Some("No Such Artist".to_string()),
            album: None,
            duration_ms: Some(180_000),
            file_path: None,
            track_id: None,
        };
        let doc = app
            .get_lyrics(query)?
            .expect("plugin should resolve lyrics");
        assert!(doc
            .lines
            .iter()
            .any(|l| l.text.contains("Core cascade line")));

        let events = drain_events(&mut rx);
        assert!(
            events.iter().any(|e| matches!(
                e,
                SonoraEvent::LyricsResolved { provider, .. }
                if provider == &format!("plugin:{id}")
            )),
            "missing plugin LyricsResolved event: {events:?}"
        );
        Ok(())
    }

    #[test]
    fn test_discovers_real_plugins() -> Result<()> {
        let plugins_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../plugins");
        let app = SonoraApp::in_memory(SonoraConfig::default())?;
        let found = app.discover_plugins(&plugins_root);
        assert!(
            found.contains(&"org.sonora.lrclib".to_string()),
            "lrclib plugin not discovered: {found:?}"
        );
        assert!(
            found.contains(&"org.sonora.example.lyrics".to_string()),
            "example plugin not discovered: {found:?}"
        );
        Ok(())
    }

    #[test]
    fn test_lrclib_cascade_via_app_with_fake_transport() -> Result<()> {
        use sonora_lyrics::provider::TrackLyricsQuery;
        use std::sync::Arc;

        let plugins_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../plugins");
        let app = SonoraApp::in_memory(SonoraConfig::default())?;
        let mut rx = app.subscribe_events();

        // Hermetic network: script LRCLIB answers, no real egress.
        let fake = Arc::new(sonora_plugin::FakeNetworkTransport::new());
        fake.route(
            "track_name=Xyzzy%20Fake%20Qqqq",
            sonora_plugin::FakeOutcome::Status(
                200,
                br#"{"trackName":"Xyzzy Fake Qqqq","artistName":"No Such Artist","syncedLyrics":"[00:01.00] App cascade synced\n","plainLyrics":"App cascade synced","instrumental":false}"#.to_vec(),
            ),
        );
        app.plugin_host().set_network_transport(fake);

        // Register the REAL plugin (manifest + wasm from the source tree).
        let dir = plugins_root.join("lyrics-lrclib");
        let manifest = sonora_plugin::PluginManifest::from_file(&dir.join("manifest.json"))
            .map_err(|e| SonoraError::Internal(format!("real manifest invalid: {e}")))?;
        let id = manifest.id.clone();
        app.register_plugin(manifest, dir)?;
        app.plugin_host()
            .load(&id)
            .map_err(|e| SonoraError::Internal(format!("real plugin load failed: {e}")))?;
        app.plugin_host()
            .start(&id)
            .map_err(|e| SonoraError::Internal(format!("real plugin start failed: {e}")))?;
        app.forward_plugin_events();

        let query = TrackLyricsQuery {
            title: "Xyzzy Fake Qqqq".to_string(),
            artist: Some("No Such Artist".to_string()),
            album: None,
            duration_ms: Some(180_000),
            file_path: None,
            track_id: None,
        };
        let doc = app
            .get_lyrics(query)?
            .expect("lrclib plugin should resolve");
        assert!(doc
            .lines
            .iter()
            .any(|l| l.text.contains("App cascade synced")));

        let events = drain_events(&mut rx);
        assert!(
            events.iter().any(|e| matches!(
                e,
                SonoraEvent::LyricsResolved { provider, .. }
                if provider == &format!("plugin:{id}")
            )),
            "missing plugin LyricsResolved event: {events:?}"
        );
        Ok(())
    }

    #[test]
    fn test_lrclib_fallback_on_missing_lyrics() -> Result<()> {
        use sonora_lyrics::provider::TrackLyricsQuery;
        use std::sync::Arc;

        let plugins_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../plugins");
        let app = SonoraApp::in_memory(SonoraConfig::default())?;
        let mut rx = app.subscribe_events();

        // Fake transport with no matching route: every fetch fails closed.
        let fake = Arc::new(sonora_plugin::FakeNetworkTransport::new());
        app.plugin_host().set_network_transport(fake);

        let dir = plugins_root.join("lyrics-lrclib");
        let manifest = sonora_plugin::PluginManifest::from_file(&dir.join("manifest.json"))
            .map_err(|e| SonoraError::Internal(format!("real manifest invalid: {e}")))?;
        let id = manifest.id.clone();
        app.register_plugin(manifest, dir)?;
        app.plugin_host()
            .load(&id)
            .map_err(|e| SonoraError::Internal(format!("real plugin load failed: {e}")))?;
        app.plugin_host()
            .start(&id)
            .map_err(|e| SonoraError::Internal(format!("real plugin start failed: {e}")))?;

        // Gibberish title: fake misses, built-ins (incl. live LRCLIB 404) miss.
        let query = TrackLyricsQuery {
            title: "Xyzzy Zilch Qqqq Nothing".to_string(),
            artist: Some("No Such Artist".to_string()),
            album: None,
            duration_ms: Some(180_000),
            file_path: None,
            track_id: None,
        };
        assert!(app.get_lyrics(query)?.is_none());

        let events = drain_events(&mut rx);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SonoraEvent::LyricsResolutionFailed { .. })),
            "missing LyricsResolutionFailed event: {events:?}"
        );
        // The miss never crashes the plugin.
        assert_eq!(
            app.plugin_host().state(&id),
            Some(sonora_plugin::PluginState::Running)
        );
        Ok(())
    }

    #[test]
    fn test_plugin_crash_isolated_via_app() -> Result<()> {
        use sonora_lyrics::provider::TrackLyricsQuery;

        let app = SonoraApp::in_memory(SonoraConfig::default())?;

        // Crasher: traps on every lyrics query (targeted at the fetch tail).
        let crasher_wat = test_lyrics_wat().replace(
            "(i32.const 1))\n  (func (export \"lyrics_result_ptr\")",
            "(unreachable))\n  (func (export \"lyrics_result_ptr\")",
        );
        let crasher_wasm = wat::parse_str(&crasher_wat).expect("crasher wat");
        let crash_id = "org.sonora.test.crasher";
        app.register_plugin(
            test_manifest(crash_id),
            std::path::PathBuf::from("core-test"),
        )?;
        app.plugin_host()
            .load_bytes(crash_id, &crasher_wasm)
            .map_err(|e| SonoraError::Internal(format!("plugin load failed: {e}")))?;
        app.plugin_host()
            .start(crash_id)
            .map_err(|e| SonoraError::Internal(format!("plugin start failed: {e}")))?;

        // Healthy sibling.
        let good_wasm = wat::parse_str(test_lyrics_wat()).expect("test wat");
        let good_id = "org.sonora.test.healthy";
        app.register_plugin(
            test_manifest(good_id),
            std::path::PathBuf::from("core-test"),
        )?;
        app.plugin_host()
            .load_bytes(good_id, &good_wasm)
            .map_err(|e| SonoraError::Internal(format!("plugin load failed: {e}")))?;
        app.plugin_host()
            .start(good_id)
            .map_err(|e| SonoraError::Internal(format!("plugin start failed: {e}")))?;

        let query = TrackLyricsQuery {
            title: "Xyzzy Zonk Qqqq Isolation".to_string(),
            artist: Some("No Such Artist".to_string()),
            album: None,
            duration_ms: Some(180_000),
            file_path: None,
            track_id: None,
        };
        // Force the crash deterministically before the cascade runs.
        let direct = sonora_plugin::LyricsQueryDto {
            title: query.title.clone(),
            artist: query.artist.clone(),
            album: None,
            duration_ms: query.duration_ms,
        };
        let _ = app.plugin_host().fetch_lyrics(crash_id, &direct);
        assert!(matches!(
            app.plugin_host().state(crash_id),
            Some(sonora_plugin::PluginState::Crashed { .. })
        ));

        let doc = app
            .get_lyrics(query)?
            .expect("healthy plugin should resolve");
        assert!(doc
            .lines
            .iter()
            .any(|l| l.text.contains("Core cascade line")));

        // Crasher is quarantined; the app and sibling are unaffected.
        assert!(matches!(
            app.plugin_host().state(crash_id),
            Some(sonora_plugin::PluginState::Crashed { .. })
        ));
        assert_eq!(
            app.plugin_host().state(good_id),
            Some(sonora_plugin::PluginState::Running)
        );
        let status = app.status();
        assert_eq!(status.queue_length, 0);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Marketplace through the app command plane
    // ------------------------------------------------------------------

    fn zip_writer(buf: &mut Vec<u8>) -> zip::ZipWriter<std::io::Cursor<&mut Vec<u8>>> {
        zip::ZipWriter::new(std::io::Cursor::new(buf))
    }

    fn write_zip_file<W: std::io::Write + std::io::Seek>(
        w: &mut zip::ZipWriter<W>,
        name: &str,
        data: &[u8],
    ) {
        use std::io::Write as _;
        w.start_file(name, zip::write::SimpleFileOptions::default())
            .expect("zip entry");
        w.write_all(data).expect("zip write");
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        use sha2::Digest as _;
        sha2::Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn market_fixture(id: &str, version: &str) -> (String, Vec<u8>) {
        let manifest = format!(
            "{{\"id\":\"{id}\",\"name\":\"Market Plugin\",\"version\":\"{version}\",\"author\":\"Tester\",\"description\":\"Market fixture.\",\"api_version\":1,\"capabilities\":[\"storage:cache\"],\"entry\":\"plugin.wasm\",\"allowed_domains\":[]}}"
        );
        let wasm = wat::parse_str(
            r#"(module
      (memory (export "memory") 1)
      (func (export "plugin_api_version") (result i32) (i32.const 1))
      (func (export "plugin_init") (result i32) (i32.const 0)))"#,
        )
        .expect("fixture wat");
        let mut buf = Vec::new();
        {
            let mut w = zip_writer(&mut buf);
            write_zip_file(&mut w, "manifest.json", manifest.as_bytes());
            write_zip_file(&mut w, "plugin.wasm", &wasm);
            w.finish().expect("finish zip");
        }
        (manifest, buf)
    }

    #[test]
    fn test_marketplace_install_update_rollback_via_app() -> Result<()> {
        use sonora_core::SonoraCommand;
        use std::sync::Arc;

        let dir = std::env::temp_dir().join(format!("sonora-mkt-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let config = SonoraConfig {
            data_dir: dir.clone(),
            ..SonoraConfig::default()
        };
        let app = SonoraApp::new(config)?;
        // Hermetic registry: scripted index + packages, no network.
        let fake = Arc::new(sonora_plugin::FakeNetworkTransport::new());
        app.marketplace().set_transport(fake.clone());

        let id = "org.sonora.test.market";
        let (_manifest_v1, zip_v1) = market_fixture(id, "1.0.0");
        let sha_v1 = sha256_hex(&zip_v1);
        // Build v2 with a marker file so versions differ on disk.
        let wasm_v2 = wat::parse_str(
            r#"(module
      (memory (export "memory") 1)
      (func (export "plugin_api_version") (result i32) (i32.const 1))
      (func (export "plugin_init") (result i32) (i32.const 0)))"#,
        )
        .expect("fixture wat");
        let manifest_v2 = _manifest_v1.replace("1.0.0", "1.1.0");
        let mut buf_v2 = Vec::new();
        {
            let mut w = zip_writer(&mut buf_v2);
            write_zip_file(&mut w, "manifest.json", manifest_v2.as_bytes());
            write_zip_file(&mut w, "plugin.wasm", &wasm_v2);
            write_zip_file(&mut w, "v2.txt", b"v2");
            w.finish().expect("finish zip");
        }
        let sha_v2 = sha256_hex(&buf_v2);

        let index = serde_json::json!({
            "schema": 1,
            "plugins": [{
                "id": id,
                "name": "Market Plugin",
                "description": "Market fixture.",
                "author": {"name": "Tester"},
                "category": "utility",
                "capabilities": ["storage:cache"],
                "api_version": 1,
                "min_sonora_version": "0.1.0",
                "latest_version": "1.1.0",
                "versions": [
                    {"version": "1.0.0", "download_url": "https://cdn.test/m-1.0.0.zip", "sha256": sha_v1, "changelog": "First."},
                    {"version": "1.1.0", "download_url": "https://cdn.test/m-1.1.0.zip", "sha256": sha_v2, "changelog": "Second."}
                ]
            }]
        })
        .to_string();
        fake.route(
            "plugins.json",
            sonora_plugin::FakeOutcome::Status(200, index.into_bytes()),
        );
        fake.route(
            "themes.json",
            sonora_plugin::FakeOutcome::Status(404, vec![]),
        );
        fake.route(
            "m-1.0.0.zip",
            sonora_plugin::FakeOutcome::Status(200, zip_v1),
        );
        fake.route(
            "m-1.1.0.zip",
            sonora_plugin::FakeOutcome::Status(200, buf_v2),
        );

        // Install explicitly at 1.0.0 through the command plane.
        app.handle_command(SonoraCommand::MarketInstall {
            id: id.to_string(),
            version: Some("1.0.0".to_string()),
        })?;
        assert_eq!(
            app.plugin_host().state(id),
            Some(sonora_plugin::PluginState::Running)
        );

        // Update to 1.1.0, then roll back.
        let updates = app.market_updates()?;
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].available, "1.1.0");
        app.handle_command(SonoraCommand::MarketUpdate { id: id.to_string() })?;
        assert!(dir.join("plugins").join(id).join("v2.txt").is_file());

        app.handle_command(SonoraCommand::MarketRollback {
            id: id.to_string(),
            version: None,
        })?;
        assert!(!dir.join("plugins").join(id).join("v2.txt").exists());
        let installed = app.market_installed()?;
        assert_eq!(installed[0].version, "1.0.0");

        // Catalog reflects install state; uninstall removes it.
        let catalog = app.market_catalog()?;
        assert_eq!(
            catalog.plugins[0].installed_version.as_deref(),
            Some("1.0.0")
        );
        app.handle_command(SonoraCommand::MarketUninstall { id: id.to_string() })?;
        assert!(app.market_installed()?.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}

#[test]
fn test_decode_all_supported_audio_formats() {
    let files = [
        "/tmp/sonora_test_audio/test_440hz.wav",
        "/tmp/sonora_test_audio/test_440hz.flac",
        "/tmp/sonora_test_audio/test_440hz.mp3",
        "/tmp/sonora_test_audio/test_440hz.ogg",
        "/tmp/sonora_test_audio/test_440hz.m4a",
        "/tmp/sonora_test_audio/test_440hz_alac.m4a",
        "/tmp/sonora_test_audio/test_96k_24bit.flac",
        "/tmp/sonora_test_audio/test_mono.wav",
    ];

    for path in &files {
        println!("Testing decoder for {path}...");
        let mut decoder = sonora_audio::AudioDecoder::open(path)
            .unwrap_or_else(|e| panic!("Failed to open {path}: {e}"));
        let info = decoder.info().clone();
        println!(
            "  Stream info: sample_rate={}, channels={}",
            info.sample_rate, info.channels
        );
        assert!(info.sample_rate > 0);

        let mut total_samples = 0;
        let mut packets = 0;
        while let Ok(Some(samples)) = decoder.decode_next_stereo() {
            assert_eq!(samples.len() % 2, 0, "Samples must be interleaved stereo");
            total_samples += samples.len();
            packets += 1;
        }
        println!("  Decoded {total_samples} samples across {packets} packets");
        assert!(total_samples > 0, "File {path} produced 0 samples!");

        // Test seek to 1 second
        decoder.seek(1000).expect("Seek to 1s must succeed");
        let mut after_seek_samples = 0;
        while let Ok(Some(samples)) = decoder.decode_next_stereo() {
            after_seek_samples += samples.len();
        }
        println!("  After seek to 1s: decoded {after_seek_samples} samples");
        assert!(
            after_seek_samples > 0,
            "Decoding after seek produced 0 samples"
        );
        assert!(
            after_seek_samples < total_samples,
            "After seek samples should be fewer than total"
        );
    }
}
