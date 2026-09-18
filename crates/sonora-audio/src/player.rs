use crate::decoder::AudioDecoder;
use crate::engine::{AudioEngine, DEFAULT_RING_BUFFER_CAPACITY};
use rtrb::RingBuffer;
use sonora_common::{PlaybackState, Result, SonoraError};
use sonora_dsp::LinearResampler;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
pub enum PlayerCommand {
    Play(PathBuf),
    Pause,
    Resume,
    Seek(u64), // position in ms
    SetVolume(f32),
    Stop,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerStateSnapshot {
    pub state: PlaybackState,
    pub current_path: Option<PathBuf>,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub volume: f32,
    pub is_finished: bool,
}

struct PlayerSharedData {
    state: PlaybackState,
    current_path: Option<PathBuf>,
    duration_ms: u64,
    is_finished: bool,
}

/// High-level audio player coordinating decoding outside the real-time audio thread
/// and feeding the lock-free output engine.
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

    pub fn pause(&self) -> Result<()> {
        self.cmd_tx
            .send(PlayerCommand::Pause)
            .map_err(|e| SonoraError::Audio(format!("Failed to send Pause command: {e}")))
    }

    pub fn resume(&self) -> Result<()> {
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

    pub fn stop(&self) -> Result<()> {
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

    pub fn visualizer_data(&self) -> Vec<f32> {
        self.engine.visualizer_data()
    }

    pub fn snapshot(&self) -> PlayerStateSnapshot {
        let shared = self.shared.lock().unwrap();
        PlayerStateSnapshot {
            state: shared.state,
            current_path: shared.current_path.clone(),
            duration_ms: shared.duration_ms,
            position_ms: self.engine.position_ms(),
            volume: self.engine.volume(),
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
        let mut resampler =
            LinearResampler::new(engine.sample_rate() as f32, engine.sample_rate() as f32);
        let mut resample_out = vec![0.0f32; 8192];
        let mut pending_buffer: Vec<f32> = Vec::with_capacity(16384);
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

                            engine.flush();
                            engine.set_frames_played(0);
                            engine.set_playing(true);
                            eof_reached = false;
                            consecutive_errors = 0;
                            pending_buffer.clear();
                            pending_offset = 0;

                            {
                                let mut s = shared.lock().unwrap();
                                s.state = PlaybackState::Playing;
                                s.current_path = Some(path);
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
                    PlayerCommand::Stop => {
                        engine.pause();
                        engine.flush();
                        current_decoder = None;
                        pending_buffer.clear();
                        pending_offset = 0;
                        let mut s = shared.lock().unwrap();
                        s.state = PlaybackState::Stopped;
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
                                        + 512)
                                        .max(1024);
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
                                    eof_reached = true;
                                    consecutive_errors = 0;
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
