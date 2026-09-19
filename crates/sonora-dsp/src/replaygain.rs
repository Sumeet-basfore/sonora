use crate::gain::{db_to_linear, Gain};
use serde::{Deserialize, Serialize};

/// Playback mode for ReplayGain loudness normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReplayGainMode {
    #[default]
    Track,
    Album,
    Disabled,
}

/// Metadata extracted from audio tags (FLAC Vorbis comments, ID3v2, MP4/M4A).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ReplayGainMetadata {
    pub track_gain_db: Option<f32>,
    pub track_peak: Option<f32>,
    pub album_gain_db: Option<f32>,
    pub album_peak: Option<f32>,
}

/// Configuration settings for ReplayGain processing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReplayGainConfig {
    pub mode: ReplayGainMode,
    pub preamp_db: f32,
    pub fallback_gain_db: f32,
    pub prevent_clipping: bool,
}

impl Default for ReplayGainConfig {
    fn default() -> Self {
        Self {
            mode: ReplayGainMode::Track,
            preamp_db: 0.0,
            fallback_gain_db: -6.0,
            prevent_clipping: true,
        }
    }
}

/// Dynamic ReplayGain / EBU R128 loudness processor with smooth gain transition.
#[derive(Debug, Clone)]
pub struct ReplayGainProcessor {
    config: ReplayGainConfig,
    metadata: ReplayGainMetadata,
    gain_interpolator: Gain,
    current_linear_gain: f32,
}

impl ReplayGainProcessor {
    pub fn new(config: ReplayGainConfig) -> Self {
        Self {
            config,
            metadata: ReplayGainMetadata::default(),
            gain_interpolator: Gain::new(1.0),
            current_linear_gain: 1.0,
        }
    }

    pub fn config(&self) -> &ReplayGainConfig {
        &self.config
    }

    pub fn metadata(&self) -> &ReplayGainMetadata {
        &self.metadata
    }

    pub fn set_config(&mut self, config: ReplayGainConfig) {
        self.config = config;
    }

    /// Calculate effective linear gain scalar based on configuration and track metadata.
    pub fn compute_effective_gain(config: &ReplayGainConfig, meta: &ReplayGainMetadata) -> f32 {
        if config.mode == ReplayGainMode::Disabled {
            return 1.0;
        }

        let (gain_db, peak_opt) = match config.mode {
            ReplayGainMode::Track => (
                meta.track_gain_db.unwrap_or(config.fallback_gain_db),
                meta.track_peak,
            ),
            ReplayGainMode::Album => {
                let g = meta
                    .album_gain_db
                    .or(meta.track_gain_db)
                    .unwrap_or(config.fallback_gain_db);
                let p = meta.album_peak.or(meta.track_peak);
                (g, p)
            }
            ReplayGainMode::Disabled => (0.0, None),
        };

        let total_db = gain_db + config.preamp_db;
        let mut linear = db_to_linear(total_db);

        if config.prevent_clipping {
            let peak = peak_opt.unwrap_or(1.0).max(0.01);
            if linear * peak > 1.0 {
                linear = 1.0 / peak;
            }
        }

        linear.clamp(0.0, 10.0)
    }

    /// Update the current track metadata and smoothly interpolate to the new target gain.
    pub fn set_track_metadata(&mut self, meta: &ReplayGainMetadata) {
        self.metadata = *meta;
        let target = Self::compute_effective_gain(&self.config, meta);
        self.current_linear_gain = target;
        self.gain_interpolator.set_target(target);
    }

    #[inline(always)]
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        self.gain_interpolator.process_sample(sample)
    }

    #[inline(always)]
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        let g = self.gain_interpolator.process_sample(1.0);
        (left * g, right * g)
    }

    pub fn reset(&mut self) {
        self.gain_interpolator = Gain::new(self.current_linear_gain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replaygain_track_mode_calculation() {
        let config = ReplayGainConfig {
            mode: ReplayGainMode::Track,
            preamp_db: 0.0,
            fallback_gain_db: -6.0,
            prevent_clipping: true,
        };

        let meta = ReplayGainMetadata {
            track_gain_db: Some(-3.0),
            track_peak: Some(0.9),
            album_gain_db: Some(-5.0),
            album_peak: Some(0.95),
        };

        let linear = ReplayGainProcessor::compute_effective_gain(&config, &meta);
        let expected = 10.0f32.powf(-3.0 / 20.0);
        assert!((linear - expected).abs() < 1e-4);
    }

    #[test]
    fn test_replaygain_prevents_clipping() {
        let config = ReplayGainConfig {
            mode: ReplayGainMode::Track,
            preamp_db: 6.0, // +6dB boost
            fallback_gain_db: 0.0,
            prevent_clipping: true,
        };

        let meta = ReplayGainMetadata {
            track_gain_db: Some(3.0),
            track_peak: Some(0.9), // Peak * (+9dB = 2.818) = 2.53 > 1.0
            album_gain_db: None,
            album_peak: None,
        };

        let linear = ReplayGainProcessor::compute_effective_gain(&config, &meta);
        // Effective gain should be clamped to 1.0 / peak = 1.0 / 0.9 = 1.111
        let max_safe = 1.0 / 0.9;
        assert!((linear - max_safe).abs() < 1e-3);
    }
}
