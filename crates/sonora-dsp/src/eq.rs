use crate::biquad::{BiquadCoefficients, BiquadFilter};
use serde::{Deserialize, Serialize};

pub const NUM_EQ_BANDS: usize = 10;

/// Standard 10-band ISO frequencies in Hz.
pub const ISO_10_BAND_FREQUENCIES: [f32; NUM_EQ_BANDS] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Configuration for a single parametric EQ band.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EqBandConfig {
    pub frequency: f32,
    pub q: f32,
    pub gain_db: f32,
    pub enabled: bool,
}

impl Default for EqBandConfig {
    fn default() -> Self {
        Self {
            frequency: 1000.0,
            q: 1.414,
            gain_db: 0.0,
            enabled: true,
        }
    }
}

/// 10-Band Parametric Equalizer Cascade.
/// Zero-allocation processing loop safe for real-time audio threads.
#[derive(Debug, Clone)]
pub struct ParametricEqualizer {
    bands: [BiquadFilter; NUM_EQ_BANDS],
    configs: [EqBandConfig; NUM_EQ_BANDS],
    sample_rate: f32,
    enabled: bool,
}

impl ParametricEqualizer {
    pub fn new(sample_rate: f32) -> Self {
        let mut configs = [EqBandConfig::default(); NUM_EQ_BANDS];
        let mut bands = [BiquadFilter::default(); NUM_EQ_BANDS];

        for i in 0..NUM_EQ_BANDS {
            configs[i].frequency = ISO_10_BAND_FREQUENCIES[i];
            let coeffs = BiquadCoefficients::peaking_eq(
                sample_rate,
                configs[i].frequency,
                configs[i].q,
                configs[i].gain_db,
            );
            bands[i] = BiquadFilter::new(coeffs);
        }

        Self {
            bands,
            configs,
            sample_rate,
            enabled: true,
        }
    }

    pub fn set_band(&mut self, band_idx: usize, gain_db: f32) {
        if band_idx < NUM_EQ_BANDS {
            self.configs[band_idx].gain_db = gain_db;
            let coeffs = BiquadCoefficients::peaking_eq(
                self.sample_rate,
                self.configs[band_idx].frequency,
                self.configs[band_idx].q,
                gain_db,
            );
            self.bands[band_idx].coeffs = coeffs;
        }
    }

    pub fn configs(&self) -> &[EqBandConfig; NUM_EQ_BANDS] {
        &self.configs
    }

    pub fn bands(&self) -> &[BiquadFilter; NUM_EQ_BANDS] {
        &self.bands
    }

    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self, mut sample: f32) -> f32 {
        if !self.enabled {
            return sample;
        }

        let all_flat = self.configs.iter().all(|c| c.gain_db.abs() < 1e-4);
        if all_flat {
            return sample;
        }

        for (i, band) in self.bands.iter_mut().enumerate() {
            if self.configs[i].enabled && self.configs[i].gain_db.abs() >= 1e-4 {
                sample = band.process_sample(sample);
            }
        }
        sample
    }
}
