use crate::decoder::AudioDecoder;
use crate::engine::{AudioEngine, DEFAULT_RING_BUFFER_CAPACITY};
use rtrb::RingBuffer;
use sonora_common::{PlaybackState, Result, SonoraError};
use sonora_dsp::{PlaybackMode, ReplayGainConfig, SincResampler};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
pub enum PlayerCommand {
    Play(PathBuf),
    EnqueueNext(PathBuf),
    Pause,
    Resume,
    Seek(u64), // position in ms
    SetVolume(f32),
    SetPlaybackMode(PlaybackMode),
    SetReplayGainConfig(ReplayGainConfig),
    SetEqBand(usize, f32),
    Stop,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerStateSnapshot {
    pub state: PlaybackState,
    pub current_path: Option<PathBuf>,
    pub next_path: Option<PathBuf>,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub volume: f32,
    pub mode: PlaybackMode,
    pub is_bit_perfect: bool,
    pub is_finished: bool,
}

struct PlayerSharedData {
    state: PlaybackState,
    current_path: Option<PathBuf>,
    next_path: Option<PathBuf>,
    duration_ms: u64,
    is_finished: bool,
}

/// High-level audio player coordinating dual-decoder prebuffering outside the real-time audio thread
/// and feeding the lock-free output engine with seamless gapless transitions.
pub struct AudioPlayer {
    engine: Arc<AudioEngine>,
    cmd_tx: Sender<PlayerCommand>,
    shared: Arc<Mutex<PlayerSharedData>>,
    shutdown: Arc<AtomicBool>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let engine = AudioEngine::new()?;
        Self::init(engine)
    }

    pub fn virtual_player(sample_rate: u32, channels: u16) -> Result<Self> {
        let engine = AudioEngine::virtual_engine(sample_rate, channels);
        Self::init(engine)
    }

    fn init(mut engine: AudioEngine) -> Result<Self> {
        let (mut producer, consumer) = RingBuffer::<f32>::new(DEFAULT_RING_BUFFER_CAPACITY);
        engine.start(consumer)?;

        let engine = Arc::new(engine);
        let (cmd_tx, cmd_rx) = channel::<PlayerCommand>();
        let shared = Arc::new(Mutex::new(PlayerSharedData {
            state: PlaybackState::Stopped,
            current_path: None,
            next_path: None,
            duration_ms: 0,
            is_finished: false,
        }));

        let shutdown = Arc::new(AtomicBool::new(false));

        let engine_clone = engine.clone();
        let shared_clone = shared.clone();
        let shutdown_clone = shutdown.clone();

        std::thread::Builder::new()
            .name("sonora-decoder-worker".to_string())
            .spawn(move || {
                Self::worker_loop(
                    engine_clone,
                    &mut producer,
                    cmd_rx,
                    shared_clone,
                    shutdown_clone,
                );
            })
            .map_err(|e| SonoraError::Internal(format!("Failed to spawn decoder thread: {e}")))?;

        Ok(Self {
            engine,
            cmd_tx,
            shared,
            shutdown,
        })
    }

    pub fn play_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path_buf = path.as_ref().to_path_buf();
        self.cmd_tx
            .send(PlayerCommand::Play(path_buf))
            .map_err(|e| SonoraError::Audio(format!("Failed to send Play command: {e}")))
    }

    pub fn enqueue_next<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path_buf = path.as_ref().to_path_buf();
        self.cmd_tx
            .send(PlayerCommand::EnqueueNext(path_buf))
            .map_err(|e| SonoraError::Audio(format!("Failed to send EnqueueNext command: {e}")))
    }

    pub fn pause(&self) -> Result<()> {
        self.engine.pause();
        {
            let mut s = self.shared.lock().unwrap();
            s.state = PlaybackState::Paused;
        }
        self.cmd_tx
            .send(PlayerCommand::Pause)
            .map_err(|e| SonoraError::Audio(format!("Failed to send Pause command: {e}")))
    }

    pub fn resume(&self) -> Result<()> {
        self.engine.resume();
        {
            let mut s = self.shared.lock().unwrap();
            s.state = PlaybackState::Playing;
        }
        self.cmd_tx
            .send(PlayerCommand::Resume)
            .map_err(|e| SonoraError::Audio(format!("Failed to send Resume command: {e}")))
    }

    pub fn seek(&self, position_ms: u64) -> Result<()> {
        self.cmd_tx
            .send(PlayerCommand::Seek(position_ms))
            .map_err(|e| SonoraError::Audio(format!("Failed to send Seek command: {e}")))
    }

    pub fn set_volume(&self, volume: f32) -> Result<()> {
        self.engine.set_volume(volume);
        self.cmd_tx
            .send(PlayerCommand::SetVolume(volume))
            .map_err(|e| SonoraError::Audio(format!("Failed to send SetVolume command: {e}")))
    }

    pub fn set_playback_mode(&self, mode: PlaybackMode) -> Result<()> {
        self.engine.set_playback_mode(mode);
        self.cmd_tx
            .send(PlayerCommand::SetPlaybackMode(mode))
            .map_err(|e| SonoraError::Audio(format!("Failed to send SetPlaybackMode command: {e}")))
    }

    pub fn set_replaygain_config(&self, config: ReplayGainConfig) -> Result<()> {
        self.engine.set_replaygain_config(config);
        self.cmd_tx
            .send(PlayerCommand::SetReplayGainConfig(config))
            .map_err(|e| {
                SonoraError::Audio(format!("Failed to send SetReplayGainConfig command: {e}"))
            })
    }

    pub fn set_eq_band(&self, band_idx: usize, gain_db: f32) -> Result<()> {
        self.engine.set_eq_band(band_idx, gain_db);
        self.cmd_tx
            .send(PlayerCommand::SetEqBand(band_idx, gain_db))
            .map_err(|e| SonoraError::Audio(format!("Failed to send SetEqBand command: {e}")))
    }

    pub fn stop(&self) -> Result<()> {
        self.engine.pause();
        self.engine.flush();
        {
            let mut s = self.shared.lock().unwrap();
            s.state = PlaybackState::Stopped;
        }
        self.cmd_tx
            .send(PlayerCommand::Stop)
            .map_err(|e| SonoraError::Audio(format!("Failed to send Stop command: {e}")))
    }

    pub fn volume(&self) -> f32 {
        self.engine.volume()
    }

    pub fn position_ms(&self) -> u64 {
        self.engine.position_ms()
    }

    pub fn playback_mode(&self) -> PlaybackMode {
        self.engine.playback_mode()
    }

    pub fn visualizer_data(&self) -> Vec<f32> {
        self.engine.visualizer_data()
    }

    pub fn snapshot(&self) -> PlayerStateSnapshot {
        let shared = self.shared.lock().unwrap();
        let mode = self.engine.playback_mode();
        let is_bit_perfect = mode == PlaybackMode::BitPerfect;
        PlayerStateSnapshot {
            state: shared.state,
            current_path: shared.current_path.clone(),
            next_path: shared.next_path.clone(),
            duration_ms: shared.duration_ms,
            position_ms: self.engine.position_ms(),
            volume: self.engine.volume(),
            mode,
            is_bit_perfect,
            is_finished: shared.is_finished,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.shared.lock().unwrap().is_finished
    }

    pub fn state(&self) -> PlaybackState {
        self.shared.lock().unwrap().state
    }

    fn worker_loop(
        engine: Arc<AudioEngine>,
        producer: &mut rtrb::Producer<f32>,
        cmd_rx: Receiver<PlayerCommand>,
        shared: Arc<Mutex<PlayerSharedData>>,
        shutdown: Arc<AtomicBool>,
    ) {
        let mut current_decoder: Option<AudioDecoder> = None;
        let mut next_decoder: Option<AudioDecoder> = None;
        let mut next_path: Option<PathBuf> = None;

        let mut resampler =
            SincResampler::new(engine.sample_rate() as f32, engine.sample_rate() as f32, 2);
        let mut resample_out = vec![0.0f32; 16384];
        let mut pending_buffer: Vec<f32> = Vec::with_capacity(32768);
        let mut pending_offset: usize = 0;
        let mut eof_reached = false;
        let mut consecutive_errors: usize = 0;

        while !shutdown.load(Ordering::Relaxed) {
            // Check for incoming commands
            let cmd = if current_decoder.is_none() || !engine.is_playing() {
                // When stopped or paused, block with timeout
                cmd_rx.recv_timeout(Duration::from_millis(50)).ok()
            } else {
                cmd_rx.try_recv().ok()
            };

            if let Some(cmd) = cmd {
                match cmd {
                    PlayerCommand::Play(path) => match AudioDecoder::open(&path) {
                        Ok(decoder) => {
                            let file_rate = decoder.info().sample_rate as f32;
                            let engine_rate = engine.sample_rate() as f32;
                            resampler.set_rates(file_rate, engine_rate);
                            resampler.reset();

                            let duration_ms = decoder
                                .info()
                                .duration_frames
                                .map(|f| (f * 1000) / decoder.info().sample_rate as u64)
                                .unwrap_or(0);

                            // Apply track ReplayGain metadata to engine
                            engine.set_track_metadata(decoder.replaygain());

                            engine.flush();
                            engine.set_frames_played(0);
                            engine.set_playing(true);
                            eof_reached = false;
                            consecutive_errors = 0;
                            pending_buffer.clear();
                            pending_offset = 0;
                            next_decoder = None;
                            next_path = None;

                            {
                                let mut s = shared.lock().unwrap();
                                s.state = PlaybackState::Playing;
                                s.current_path = Some(path);
                                s.next_path = None;
                                s.duration_ms = duration_ms;
                                s.is_finished = false;
                            }

                            current_decoder = Some(decoder);
                        }
                        Err(e) => {
                            tracing::error!("Failed to open track: {e}");
                            let mut s = shared.lock().unwrap();
                            s.state = PlaybackState::Stopped;
                            s.is_finished = true;
                        }
                    },
                    PlayerCommand::EnqueueNext(path) => match AudioDecoder::open(&path) {
                        Ok(decoder) => {
                            next_decoder = Some(decoder);
                            next_path = Some(path.clone());
                            let mut s = shared.lock().unwrap();
                            s.next_path = Some(path);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to pre-open next track for gapless queue: {e}");
                            next_decoder = None;
                            next_path = None;
                            let mut s = shared.lock().unwrap();
                            s.next_path = None;
                        }
                    },
                    PlayerCommand::Pause => {
                        engine.pause();
                        let mut s = shared.lock().unwrap();
                        s.state = PlaybackState::Paused;
                    }
                    PlayerCommand::Resume => {
                        engine.resume();
                        let mut s = shared.lock().unwrap();
                        s.state = PlaybackState::Playing;
                    }
                    PlayerCommand::Seek(pos_ms) => {
                        if let Some(decoder) = &mut current_decoder {
                            let duration_ms = decoder
                                .info()
                                .duration_frames
                                .map(|f| (f * 1000) / decoder.info().sample_rate as u64)
                                .unwrap_or(0);
                            let clamped_ms = if duration_ms > 500 {
                                pos_ms.min(duration_ms.saturating_sub(250))
                            } else {
                                pos_ms
                            };
                            if let Err(e) = decoder.seek(clamped_ms) {
                                tracing::error!("Seek failed: {e}");
                            } else {
                                resampler.reset();
                                engine.flush();
                                let target_frames =
                                    (clamped_ms * engine.sample_rate() as u64) / 1000;
                                engine.set_frames_played(target_frames);
                                eof_reached = false;
                                consecutive_errors = 0;
                                pending_buffer.clear();
                                pending_offset = 0;
                                let mut s = shared.lock().unwrap();
                                s.is_finished = false;
                            }
                        }
                    }
                    PlayerCommand::SetVolume(vol) => {
                        engine.set_volume(vol);
                    }
                    PlayerCommand::SetPlaybackMode(mode) => {
                        engine.set_playback_mode(mode);
                    }
                    PlayerCommand::SetReplayGainConfig(config) => {
                        engine.set_replaygain_config(config);
                    }
                    PlayerCommand::SetEqBand(band_idx, gain_db) => {
                        engine.set_eq_band(band_idx, gain_db);
                    }
                    PlayerCommand::Stop => {
                        engine.pause();
                        engine.flush();
                        current_decoder = None;
                        next_decoder = None;
                        next_path = None;
                        pending_buffer.clear();
                        pending_offset = 0;
                        let mut s = shared.lock().unwrap();
                        s.state = PlaybackState::Stopped;
                        s.next_path = None;
                    }
                }
            }

            // Decode and feed the ring buffer if playing
            if engine.is_playing() {
                if let Some(decoder) = &mut current_decoder {
                    // First, drain any pending samples from the previous decode/resample step
                    if pending_offset < pending_buffer.len() {
                        let available_slots = producer.slots();
                        // Only push even number of samples to preserve stereo frame alignment
                        let pairs_available = available_slots / 2;
                        let remaining_pairs = (pending_buffer.len() - pending_offset) / 2;
                        let pairs_to_push = pairs_available.min(remaining_pairs);

                        if pairs_to_push > 0 {
                            let samples_to_push = pairs_to_push * 2;
                            for &s in
                                &pending_buffer[pending_offset..pending_offset + samples_to_push]
                            {
                                let _ = producer.push(s);
                            }
                            pending_offset += samples_to_push;
                        }
                    }

                    // If pending buffer is fully drained and EOF not reached, decode next packet
                    if pending_offset >= pending_buffer.len() && !eof_reached {
                        if producer.slots() >= 1024 {
                            let in_rate = decoder.info().sample_rate;
                            match decoder.decode_next_stereo() {
                                Ok(Some(stereo_samples)) => {
                                    consecutive_errors = 0;
                                    let req_len = ((stereo_samples.len() as f32
                                        * (engine.sample_rate() as f32 / in_rate as f32))
                                        .ceil()
                                        as usize
                                        + 1024)
                                        .max(2048);
                                    if resample_out.len() < req_len {
                                        resample_out.resize(req_len, 0.0);
                                    }

                                    let frames_written = resampler
                                        .resample_stereo(stereo_samples, &mut resample_out);
                                    let samples_produced = frames_written * 2;

                                    pending_buffer.clear();
                                    pending_buffer
                                        .extend_from_slice(&resample_out[..samples_produced]);
                                    pending_offset = 0;

                                    // Push as many as fit immediately
                                    let available_slots = producer.slots();
                                    let pairs_available = available_slots / 2;
                                    let remaining_pairs = samples_produced / 2;
                                    let pairs_to_push = pairs_available.min(remaining_pairs);

                                    if pairs_to_push > 0 {
                                        let samples_to_push = pairs_to_push * 2;
                                        for &s in &pending_buffer[..samples_to_push] {
                                            let _ = producer.push(s);
                                        }
                                        pending_offset = samples_to_push;
                                    }
                                }
                                Ok(None) => {
                                    // Flush remaining samples in resampler FIFO
                                    let flush_frames = resampler.flush_remaining(&mut resample_out);
                                    if flush_frames > 0 {
                                        let flush_samples = flush_frames * 2;
                                        pending_buffer.clear();
                                        pending_buffer
                                            .extend_from_slice(&resample_out[..flush_samples]);
                                        pending_offset = 0;
                                    }

                                    // Check if we have a gapless next track prepared
                                    if let Some(next_dec) = next_decoder.take() {
                                        tracing::info!("Gapless transition: Promoting pre-buffered next track to active decoder");
                                        let file_rate = next_dec.info().sample_rate as f32;
                                        let engine_rate = engine.sample_rate() as f32;
                                        resampler.set_rates(file_rate, engine_rate);
                                        resampler.reset();

                                        let duration_ms = next_dec
                                            .info()
                                            .duration_frames
                                            .map(|f| {
                                                (f * 1000) / next_dec.info().sample_rate as u64
                                            })
                                            .unwrap_or(0);

                                        engine.set_track_metadata(next_dec.replaygain());
                                        engine.set_frames_played(0);

                                        let promoted_path = next_path.take();
                                        {
                                            let mut s = shared.lock().unwrap();
                                            s.current_path = promoted_path;
                                            s.next_path = None;
                                            s.duration_ms = duration_ms;
                                            s.is_finished = false;
                                        }

                                        current_decoder = Some(next_dec);
                                        eof_reached = false;
                                        consecutive_errors = 0;
                                    } else {
                                        eof_reached = true;
                                        consecutive_errors = 0;
                                    }
                                }
                                Err(e) => {
                                    consecutive_errors += 1;
                                    tracing::warn!("Decode error ({consecutive_errors}/5): {e}");
                                    if consecutive_errors >= 5 {
                                        tracing::error!("Exceeded max consecutive decode errors, ending playback of current file");
                                        eof_reached = true;
                                    }
                                }
                            }
                        } else {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                    } else if eof_reached && pending_offset >= pending_buffer.len() {
                        if producer.slots() == DEFAULT_RING_BUFFER_CAPACITY {
                            // All decoded samples played through CPAL
                            engine.pause();
                            let mut s = shared.lock().unwrap();
                            s.state = PlaybackState::Stopped;
                            s.is_finished = true;
                        } else {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    } else {
                        // Pending buffer still has data but ring buffer is full; yield briefly
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
            }
        }
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = self.cmd_tx.send(PlayerCommand::Stop);
    }
}
