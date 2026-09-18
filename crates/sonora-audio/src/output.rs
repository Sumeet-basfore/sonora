use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host, Stream, StreamConfig, SupportedStreamConfig};
use sonora_common::{Result, SonoraError};

/// Audio output device manager wrapping CPAL.
pub struct AudioOutput {
    _host: Host,
    device: Device,
    config: SupportedStreamConfig,
}

impl AudioOutput {
    /// Initialize audio output using the system default audio device.
    pub fn default_device() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| SonoraError::Audio("No audio output device found on system".to_string()))?;

        let config = device
            .default_output_config()
            .map_err(|e| SonoraError::Audio(format!("Failed to retrieve default audio output config: {e}")))?;

        Ok(Self {
            _host: host,
            device,
            config,
        })
    }

    pub fn device_name(&self) -> Result<String> {
        self.device
            .description()
            .map(|d| d.name().to_string())
            .map_err(|e| SonoraError::Audio(format!("Failed to get device description: {e}")))
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate()
    }

    pub fn channels(&self) -> u16 {
        self.config.channels()
    }

    /// Build a real-time output stream.
    /// Note: The audio callback is executed synchronously on the high-priority audio thread.
    pub fn build_stream<F>(&self, mut callback: F) -> Result<Stream>
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        let config: StreamConfig = self.config.clone().into();
        let err_fn = |err| {
            tracing::error!("CPAL audio stream error: {err}");
        };

        let stream = self
            .device
            .build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    callback(data);
                },
                err_fn,
                None,
            )
            .map_err(|e| SonoraError::Audio(format!("Failed to build CPAL output stream: {e}")))?;

        Ok(stream)
    }
}
