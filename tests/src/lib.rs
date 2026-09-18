#[cfg(test)]
mod tests {
    use sonora_audio::AudioRingBuffer;
    use sonora_common::{
        init_logging, LogConfig, PlaybackState, Result, SonoraError, TrackId,
    };
    use sonora_core::{SonoraApp, SonoraConfig, SonoraEvent};
    use sonora_dsp::{db_to_linear, linear_to_db, BiquadCoefficients, BiquadFilter, ParametricEqualizer};
    use sonora_library::{Database, LibraryRepository};
    use sonora_lyrics::{LyricsFormat, LyricsParser, PlainTextLyricsParser};

    #[test]
    fn test_error_handling_and_propagation() {
        let err = SonoraError::Audio("Output device disconnected".to_string());
        assert_eq!(err.to_string(), "Audio subsystem error: Output device disconnected");

        let res: Result<()> = Err(SonoraError::Database("Lock busy".to_string()));
        assert!(res.is_err());
    }

    #[test]
    fn test_logging_initialization() {
        // Safe to initialize multiple times
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
            248000,
            44100,
            2,
            "flac",
        )?;

        assert_eq!(track_id, TrackId(1));

        // Test FTS5 trigram search
        let results = repo.search("Lucky", 10)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Get Lucky");
        assert_eq!(results[0].artist_name.as_deref(), Some("Daft Punk"));
        assert_eq!(results[0].album_title.as_deref(), Some("Random Access Memories"));

        Ok(())
    }

    #[test]
    fn test_dsp_biquad_and_equalizer() {
        // Test dB <-> linear conversions
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-5);
        assert!((db_to_linear(6.0) - 1.99526).abs() < 1e-4);
        assert!((linear_to_db(1.0) - 0.0).abs() < 1e-5);

        // Test Biquad identity filter
        let mut filter = BiquadFilter::new(BiquadCoefficients::identity());
        let sample = 0.75f32;
        assert_eq!(filter.process_sample(sample), sample);

        // Test 10-band equalizer processing
        let mut eq = ParametricEqualizer::new(48000.0);
        eq.set_band(0, 3.0); // +3dB at 31Hz
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

        let received = rx.recv().await.map_err(|e| SonoraError::Internal(e.to_string()))?;
        match received {
            SonoraEvent::PlaybackStateChanged(state) => assert_eq!(state, PlaybackState::Playing),
            _ => panic!("Unexpected event received"),
        }

        Ok(())
    }
}
