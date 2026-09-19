use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig, SupportedStreamConfig};
use sonora_common::{Result, SonoraError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// Audio output manager wrapping CPAL with fallback to a virtual clock driver for headless environments.
pub enum AudioOutput {
    Cpal {
        _host: Host,
        device: Device,
        config: SupportedStreamConfig,
    },
    Virtual {
        sample_rate: u32,
        channels: u16,
    },
}

impl AudioOutput {
    /// Initialize audio output using the system default audio device, or virtual driver if unavailable.
    pub fn default_device() -> Result<Self> {
        let host = cpal::default_host();
        if let Some(device) = host.default_output_device() {
            if let Ok(config) = device.default_output_config() {
                return Ok(Self::Cpal {
                    _host: host,
                    device,
                    config,
                });
            }
        }

        tracing::warn!("No physical audio device available; falling back to virtual audio output");
        Ok(Self::Virtual {
            sample_rate: 48000,
            channels: 2,
        })
    }

    /// Initialize audio output using a specific device name or fallback to default.
    pub fn from_device_name(name: &str) -> Result<Self> {
        let host = cpal::default_host();
        if let Ok(devices) = host.output_devices() {
            for dev in devices {
                if let Ok(desc) = dev.description() {
                    if desc.name() == name {
                        if let Ok(config) = dev.default_output_config() {
                            return Ok(Self::Cpal {
                                _host: host,
                                device: dev,
                                config,
                            });
                        }
                    }
                }
            }
        }
        Self::default_device()
    }

    /// Explicitly create a virtual clock driver (for tests and headless CI).
    pub fn virtual_output(sample_rate: u32, channels: u16) -> Self {
        Self::Virtual {
            sample_rate,
            channels,
        }
    }

    pub fn device_name(&self) -> Result<String> {
        match self {
            Self::Cpal { device, .. } => device
                .description()
                .map(|d| d.name().to_string())
                .map_err(|e| SonoraError::Audio(format!("Failed to get device description: {e}"))),
            Self::Virtual { .. } => Ok("Virtual Audio Clock Output".to_string()),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        match self {
            Self::Cpal { config, .. } => config.sample_rate(),
            Self::Virtual { sample_rate, .. } => *sample_rate,
        }
    }

    pub fn channels(&self) -> u16 {
        match self {
            Self::Cpal { config, .. } => config.channels(),
            Self::Virtual { channels, .. } => *channels,
        }
    }

    /// Build a real-time output stream.
    pub fn build_stream<F>(&self, mut callback: F) -> Result<OutputStreamHandle>
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        match self {
            Self::Cpal { device, config, .. } => {
                let stream_config: StreamConfig = (*config).into();
                let last_error_log = std::sync::Mutex::new(std::time::Instant::now());
                let err_fn = move |err| {
                    if let Ok(mut last) = last_error_log.lock() {
                        if last.elapsed() > Duration::from_secs(2) {
                            tracing::warn!("CPAL audio stream status: {err}");
                            *last = std::time::Instant::now();
                        }
                    }
                };

                let stream = device
                    .build_output_stream(
                        stream_config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            callback(data);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| {
                        SonoraError::Audio(format!("Failed to build CPAL output stream: {e}"))
                    })?;

                Ok(OutputStreamHandle::Cpal(stream))
            }

            Self::Virtual {
                sample_rate,
                channels,
            } => {
                let running = Arc::new(AtomicBool::new(false));
                let running_clone = running.clone();
                let chunk_size = 512 * (*channels as usize);
                let sleep_duration =
                    Duration::from_micros((512 * 1_000_000) / (*sample_rate as u64));

                let handle = std::thread::Builder::new()
                    .name("sonora-virtual-audio".to_string())
                    .spawn(move || {
                        let mut buffer = vec![0.0f32; chunk_size];
                        while running_clone.load(Ordering::Relaxed) {
                            callback(&mut buffer);
                            std::thread::sleep(sleep_duration);
                        }
                    })
                    .map_err(|e| {
                        SonoraError::Internal(format!("Failed to spawn virtual clock: {e}"))
                    })?;

                Ok(OutputStreamHandle::Virtual {
                    running,
                    _handle: Some(handle),
                })
            }
        }
    }
}

/// Abstract handle to an active audio output stream.
pub enum OutputStreamHandle {
    Cpal(Stream),
    Virtual {
        running: Arc<AtomicBool>,
        _handle: Option<JoinHandle<()>>,
    },
}

impl OutputStreamHandle {
    pub fn play(&self) -> Result<()> {
        match self {
            Self::Cpal(stream) => stream
                .play()
                .map_err(|e| SonoraError::Audio(format!("Failed to play CPAL stream: {e}"))),
            Self::Virtual { running, .. } => {
                running.store(true, Ordering::SeqCst);
                Ok(())
            }
        }
    }

    pub fn pause(&self) -> Result<()> {
        match self {
            Self::Cpal(stream) => stream
                .pause()
                .map_err(|e| SonoraError::Audio(format!("Failed to pause CPAL stream: {e}"))),
            Self::Virtual { running, .. } => {
                running.store(false, Ordering::SeqCst);
                Ok(())
            }
        }
    }
}

impl Drop for OutputStreamHandle {
    fn drop(&mut self) {
        if let Self::Virtual { running, .. } = self {
            running.store(false, Ordering::SeqCst);
        }
    }
}
