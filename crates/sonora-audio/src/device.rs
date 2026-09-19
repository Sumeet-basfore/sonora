use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

/// Hardware audio output device metadata and capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub supported_sample_rates: Vec<u32>,
    pub min_channels: u16,
    pub max_channels: u16,
    pub is_exclusive_capable: bool,
}

/// Enumerate all available hardware audio output devices on the current host.
pub fn enumerate_output_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_output_device()
        .and_then(|d| d.description().ok().map(|desc| desc.name().to_string()));

    let mut devices = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    if let Ok(dev_iter) = host.output_devices() {
        for dev in dev_iter {
            if let Ok(desc) = dev.description() {
                let name = desc.name().to_string();
                if !seen_names.insert(name.clone()) {
                    continue;
                }

                let is_default = default_name.as_ref().map(|d| d == &name).unwrap_or(false);
                let mut rates = std::collections::BTreeSet::new();
                let mut min_ch = 2u16;
                let mut max_ch = 2u16;

                if let Ok(configs) = dev.supported_output_configs() {
                    for cfg in configs {
                        min_ch = min_ch.min(cfg.channels());
                        max_ch = max_ch.max(cfg.channels());
                        let min_r = cfg.min_sample_rate();
                        let max_r = cfg.max_sample_rate();

                        for &standard_rate in
                            &[44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000]
                        {
                            if standard_rate >= min_r && standard_rate <= max_r {
                                rates.insert(standard_rate);
                            }
                        }
                    }
                }

                if rates.is_empty() {
                    rates.insert(44100);
                    rates.insert(48000);
                }

                // Check exclusive mode capability per platform
                #[cfg(target_os = "windows")]
                let is_exclusive_capable = true; // WASAPI exclusive
                #[cfg(target_os = "linux")]
                let is_exclusive_capable = true; // ALSA hw: / PipeWire direct
                #[cfg(target_os = "macos")]
                let is_exclusive_capable = true; // CoreAudio hog mode
                #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
                let is_exclusive_capable = false;

                devices.push(AudioDevice {
                    id: format!("dev_{}", name.replace(' ', "_")),
                    name,
                    is_default,
                    supported_sample_rates: rates.into_iter().collect(),
                    min_channels: min_ch,
                    max_channels: max_ch,
                    is_exclusive_capable,
                });
            }
        }
    }

    if devices.is_empty() {
        devices.push(AudioDevice {
            id: "virtual_default".to_string(),
            name: "Default System Audio Device".to_string(),
            is_default: true,
            supported_sample_rates: vec![44100, 48000, 96000, 192000],
            min_channels: 1,
            max_channels: 2,
            is_exclusive_capable: false,
        });
    }

    devices
}

/// Retrieve the system default output device metadata.
pub fn get_default_device() -> AudioDevice {
    let devices = enumerate_output_devices();
    devices
        .into_iter()
        .find(|d| d.is_default)
        .unwrap_or_else(|| AudioDevice {
            id: "default".to_string(),
            name: "Default Audio Device".to_string(),
            is_default: true,
            supported_sample_rates: vec![44100, 48000],
            min_channels: 2,
            max_channels: 2,
            is_exclusive_capable: false,
        })
}
