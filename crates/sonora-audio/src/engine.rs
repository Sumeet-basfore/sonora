use crate::output::AudioOutput;
use cpal::traits::StreamTrait;
use cpal::Stream;
use rtrb::Consumer;
use sonora_common::Result;
use sonora_dsp::{Gain, ParametricEqualizer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Real-time audio engine coordinating output stream, lock-free ring buffer, and DSP processing.
/// Note: The audio thread render callback is strictly synchronous with zero allocations.
pub struct AudioEngine {
    output: AudioOutput,
    stream: Option<Stream>,
    is_running: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        let output = AudioOutput::default_device()?;
        Ok(Self {
            output,
            stream: None,
            is_running: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.output.sample_rate()
    }

    pub fn channels(&self) -> u16 {
        self.output.channels()
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Attach a consumer ring buffer and start playback stream.
    /// Real-time safe: callback executes on OS audio thread.
    pub fn start_stream(&mut self, mut consumer: Consumer<f32>) -> Result<()> {
        let mut eq = ParametricEqualizer::new(self.sample_rate() as f32);
        let mut gain = Gain::new(1.0);
        let is_running_clone = self.is_running.clone();

        let stream = self.output.build_stream(move |data: &mut [f32]| {
            for sample in data.iter_mut() {
                let in_sample = consumer.pop().unwrap_or(0.0);
                let eq_sample = eq.process_sample(in_sample);
                *sample = gain.process_sample(eq_sample);
            }
        })?;

        stream.play().map_err(|e| {
            sonora_common::SonoraError::Audio(format!("Failed to start CPAL stream: {e}"))
        })?;

        self.stream = Some(stream);
        is_running_clone.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            stream.pause().map_err(|e| {
                sonora_common::SonoraError::Audio(format!("Failed to pause stream: {e}"))
            })?;
            self.is_running.store(false, Ordering::SeqCst);
        }
        Ok(())
    }
}
