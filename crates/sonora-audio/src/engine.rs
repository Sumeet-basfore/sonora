use crate::output::{AudioOutput, OutputStreamHandle};
use rtrb::Consumer;
use sonora_common::Result;
use sonora_dsp::{
    DspPipeline, PlaybackMode, ReplayGainConfig, ReplayGainMetadata, SpectrumAnalyzer,
};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const DEFAULT_RING_BUFFER_CAPACITY: usize = 32768; // ~340ms of stereo audio at 48kHz
pub const VISUALIZER_SAMPLES: usize = sonora_dsp::DEFAULT_FFT_SIZE; // 1024 samples

/// Lock-free, zero-allocation visualizer audio tap on the real-time CPAL thread.
pub struct VisualizerTap {
    buffer: Vec<AtomicU32>,
    write_pos: std::sync::atomic::AtomicUsize,
}

impl Default for VisualizerTap {
    fn default() -> Self {
        let mut buffer = Vec::with_capacity(VISUALIZER_SAMPLES);
        for _ in 0..VISUALIZER_SAMPLES {
            buffer.push(AtomicU32::new(0));
        }
        Self {
            buffer,
            write_pos: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl VisualizerTap {
    #[inline]
    pub fn push(&self, sample: f32) {
        let idx = self.write_pos.fetch_add(1, Ordering::Relaxed) % VISUALIZER_SAMPLES;
        self.buffer[idx].store(sample.to_bits(), Ordering::Relaxed);
    }

    pub fn get_samples(&self) -> Vec<f32> {
        let mut samples = Vec::with_capacity(VISUALIZER_SAMPLES);
        let start = self.write_pos.load(Ordering::Relaxed);
        for i in 0..VISUALIZER_SAMPLES {
            let idx = (start + i) % VISUALIZER_SAMPLES;
            let val = f32::from_bits(self.buffer[idx].load(Ordering::Relaxed));
            samples.push(val);
        }
        samples
    }
}

/// Real-time audio engine coordinating the output stream, DSP pipeline, and lock-free ring buffer.
pub struct AudioEngine {
    output: AudioOutput,
    stream: Option<OutputStreamHandle>,
    is_playing: Arc<AtomicBool>,
    volume_bits: Arc<AtomicU32>,
    playback_mode_bits: Arc<AtomicU32>,
    frames_played: Arc<AtomicU64>,
    flush_epoch: Arc<AtomicU32>,
    visualizer_tap: Arc<VisualizerTap>,
    spectrum_analyzer: Mutex<SpectrumAnalyzer>,
    dsp_pipeline: Arc<Mutex<DspPipeline>>,
    sample_rate: u32,
    channels: u16,
}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        let output = AudioOutput::default_device()?;
        let sample_rate = output.sample_rate();
        let channels = output.channels();
        let analyzer = SpectrumAnalyzer::new(
            VISUALIZER_SAMPLES,
            sonora_dsp::DEFAULT_SPECTRUM_BANDS,
            sample_rate as f32,
        );
        let pipeline = DspPipeline::new(sample_rate as f32);

        Ok(Self {
            output,
            stream: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            volume_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            playback_mode_bits: Arc::new(AtomicU32::new(PlaybackMode::DefaultShared as u32)),
            frames_played: Arc::new(AtomicU64::new(0)),
            flush_epoch: Arc::new(AtomicU32::new(0)),
            visualizer_tap: Arc::new(VisualizerTap::default()),
            spectrum_analyzer: Mutex::new(analyzer),
            dsp_pipeline: Arc::new(Mutex::new(pipeline)),
            sample_rate,
            channels,
        })
    }

    pub fn virtual_engine(sample_rate: u32, channels: u16) -> Self {
        let output = AudioOutput::virtual_output(sample_rate, channels);
        let analyzer = SpectrumAnalyzer::new(
            VISUALIZER_SAMPLES,
            sonora_dsp::DEFAULT_SPECTRUM_BANDS,
            sample_rate as f32,
        );
        let pipeline = DspPipeline::new(sample_rate as f32);

        Self {
            output,
            stream: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            volume_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            playback_mode_bits: Arc::new(AtomicU32::new(PlaybackMode::DefaultShared as u32)),
            frames_played: Arc::new(AtomicU64::new(0)),
            flush_epoch: Arc::new(AtomicU32::new(0)),
            visualizer_tap: Arc::new(VisualizerTap::default()),
            spectrum_analyzer: Mutex::new(analyzer),
            dsp_pipeline: Arc::new(Mutex::new(pipeline)),
            sample_rate,
            channels,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    pub fn set_playing(&self, playing: bool) {
        self.is_playing.store(playing, Ordering::SeqCst);
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume_bits.load(Ordering::Relaxed))
    }

    pub fn set_volume(&self, volume: f32) {
        let clamped = volume.clamp(0.0, 2.0);
        self.volume_bits.store(clamped.to_bits(), Ordering::Relaxed);
        if let Ok(mut pipe) = self.dsp_pipeline.lock() {
            pipe.set_volume(clamped);
        }
    }

    pub fn playback_mode(&self) -> PlaybackMode {
        match self.playback_mode_bits.load(Ordering::Relaxed) {
            1 => PlaybackMode::HighQuality,
            2 => PlaybackMode::BitPerfect,
            3 => PlaybackMode::ExclusiveDsp,
            _ => PlaybackMode::DefaultShared,
        }
    }

    pub fn set_playback_mode(&self, mode: PlaybackMode) {
        let val = match mode {
            PlaybackMode::DefaultShared => 0,
            PlaybackMode::HighQuality => 1,
            PlaybackMode::BitPerfect => 2,
            PlaybackMode::ExclusiveDsp => 3,
        };
        self.playback_mode_bits.store(val, Ordering::Relaxed);
        if let Ok(mut pipe) = self.dsp_pipeline.lock() {
            pipe.set_mode(mode);
        }
    }

    pub fn set_replaygain_config(&self, config: ReplayGainConfig) {
        if let Ok(mut pipe) = self.dsp_pipeline.lock() {
            pipe.replay_gain_mut().set_config(config);
        }
    }

    pub fn set_track_metadata(&self, meta: &ReplayGainMetadata) {
        if let Ok(mut pipe) = self.dsp_pipeline.lock() {
            pipe.set_track_metadata(meta);
        }
    }

    pub fn set_eq_band(&self, band_idx: usize, gain_db: f32) {
        if let Ok(mut pipe) = self.dsp_pipeline.lock() {
            pipe.set_eq_band(band_idx, gain_db);
        }
    }

    pub fn frames_played(&self) -> u64 {
        self.frames_played.load(Ordering::Relaxed)
    }

    pub fn set_frames_played(&self, frames: u64) {
        self.frames_played.store(frames, Ordering::Relaxed);
    }

    pub fn position_ms(&self) -> u64 {
        let frames = self.frames_played();
        if self.sample_rate > 0 {
            (frames * 1000) / self.sample_rate as u64
        } else {
            0
        }
    }

    pub fn visualizer_data(&self) -> Vec<f32> {
        if !self.is_playing() {
            vec![0.0; sonora_dsp::DEFAULT_SPECTRUM_BANDS]
        } else {
            let samples = self.visualizer_tap.get_samples();
            if let Ok(mut analyzer) = self.spectrum_analyzer.lock() {
                analyzer.set_sample_rate(self.sample_rate as f32);
                analyzer.compute_spectrum(&samples)
            } else {
                vec![0.0; sonora_dsp::DEFAULT_SPECTRUM_BANDS]
            }
        }
    }

    /// Flush all buffered samples from the real-time audio pipeline.
    pub fn flush(&self) {
        self.flush_epoch.fetch_add(1, Ordering::SeqCst);
    }

    /// Initialize the output stream connected to the consumer ring buffer.
    pub fn start(&mut self, mut consumer: Consumer<f32>) -> Result<()> {
        let is_playing = self.is_playing.clone();
        let volume_bits = self.volume_bits.clone();
        let playback_mode_bits = self.playback_mode_bits.clone();
        let frames_played = self.frames_played.clone();
        let flush_epoch = self.flush_epoch.clone();
        let tap = self.visualizer_tap.clone();
        let pipeline_arc = self.dsp_pipeline.clone();
        let channels = self.channels as usize;

        let mut local_flush = flush_epoch.load(Ordering::Relaxed);
        let mut local_pipeline = DspPipeline::new(self.sample_rate as f32);

        let stream = self.output.build_stream(move |data: &mut [f32]| {
            // Check for flush request
            let current_flush = flush_epoch.load(Ordering::Relaxed);
            if current_flush != local_flush {
                while consumer.pop().is_ok() {}
                local_flush = current_flush;
            }

            let playing = is_playing.load(Ordering::Relaxed);
            let target_volume = f32::from_bits(volume_bits.load(Ordering::Relaxed));
            let mode_val = playback_mode_bits.load(Ordering::Relaxed);
            let mode = match mode_val {
                1 => PlaybackMode::HighQuality,
                2 => PlaybackMode::BitPerfect,
                3 => PlaybackMode::ExclusiveDsp,
                _ => PlaybackMode::DefaultShared,
            };

            local_pipeline.set_mode(mode);
            local_pipeline.set_volume(target_volume);

            if let Ok(pipe_lock) = pipeline_arc.try_lock() {
                // Copy EQ bands if changed
                for (i, config) in pipe_lock.eq_left().configs().iter().enumerate() {
                    local_pipeline.set_eq_band(i, config.gain_db);
                }
                local_pipeline
                    .replay_gain_mut()
                    .set_config(*pipe_lock.replay_gain().config());
                local_pipeline.set_track_metadata(pipe_lock.replay_gain().metadata());
            }

            if !playing {
                data.fill(0.0);
                return;
            }

            let num_channels = channels.max(1);
            for chunk in data.chunks_exact_mut(num_channels) {
                let raw_l = consumer.pop().unwrap_or(0.0);
                let raw_r = consumer.pop().unwrap_or(0.0);

                let (out_l, out_r) = local_pipeline.process_stereo(raw_l, raw_r);

                if num_channels == 1 {
                    chunk[0] = (out_l + out_r) * 0.5;
                } else {
                    chunk[0] = out_l;
                    chunk[1] = out_r;
                    for sample in &mut chunk[2..num_channels] {
                        *sample = 0.0;
                    }
                }
                frames_played.fetch_add(1, Ordering::Relaxed);
                tap.push((out_l + out_r) * 0.5);
            }
        })?;

        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn pause(&self) {
        self.set_playing(false);
    }

    pub fn resume(&self) {
        self.set_playing(true);
    }
}
