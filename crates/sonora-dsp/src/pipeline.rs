use crate::dither::{DitherMode, TpdfDither};
use crate::eq::ParametricEqualizer;
use crate::gain::{db_to_linear, Gain};
use crate::limiter::{Limiter, TruePeakLimiter};
use crate::replaygain::{ReplayGainConfig, ReplayGainMetadata, ReplayGainProcessor};
use serde::{Deserialize, Serialize};

/// The four core playback modes defined in the Sonora Audio Engine specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlaybackMode {
    #[default]
    DefaultShared,
    HighQuality,
    BitPerfect,
    ExclusiveDsp,
}

/// Unified DSP signal chain coordinating ReplayGain -> Headroom -> EQ -> Volume -> True-Peak Limiter -> Dither.
#[derive(Debug, Clone)]
pub struct DspPipeline {
    sample_rate: f32,
    mode: PlaybackMode,
    replay_gain: ReplayGainProcessor,
    eq_left: ParametricEqualizer,
    eq_right: ParametricEqualizer,
    headroom_gain: f32,
    master_gain: Gain,
    limiter: TruePeakLimiter,
    dither: TpdfDither,
    volume: f32,
}

impl DspPipeline {
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = sample_rate.max(8000.0);
        let mut p = Self {
            sample_rate,
            mode: PlaybackMode::DefaultShared,
            replay_gain: ReplayGainProcessor::new(ReplayGainConfig::default()),
            eq_left: ParametricEqualizer::new(sample_rate),
            eq_right: ParametricEqualizer::new(sample_rate),
            headroom_gain: 1.0,
            master_gain: Gain::new(1.0),
            limiter: TruePeakLimiter::new(sample_rate, -0.3),
            dither: TpdfDither::new(DitherMode::None),
            volume: 1.0,
        };
        p.update_headroom();
        p
    }

    pub fn mode(&self) -> PlaybackMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: PlaybackMode) {
        self.mode = mode;
        if mode == PlaybackMode::BitPerfect {
            self.master_gain.set_target(1.0);
        } else {
            self.master_gain.set_target(self.volume);
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() >= 1.0 {
            self.sample_rate = sample_rate.max(8000.0);
            self.limiter.set_sample_rate(self.sample_rate);
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 2.0);
        if self.mode != PlaybackMode::BitPerfect {
            self.master_gain.set_target(self.volume);
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn replay_gain(&self) -> &ReplayGainProcessor {
        &self.replay_gain
    }

    pub fn replay_gain_mut(&mut self) -> &mut ReplayGainProcessor {
        &mut self.replay_gain
    }

    pub fn eq_left(&self) -> &ParametricEqualizer {
        &self.eq_left
    }

    pub fn eq_left_mut(&mut self) -> &mut ParametricEqualizer {
        &mut self.eq_left
    }

    pub fn eq_right(&self) -> &ParametricEqualizer {
        &self.eq_right
    }

    pub fn eq_right_mut(&mut self) -> &mut ParametricEqualizer {
        &mut self.eq_right
    }

    pub fn set_eq_band(&mut self, band_idx: usize, gain_db: f32) {
        self.eq_left.set_band(band_idx, gain_db);
        self.eq_right.set_band(band_idx, gain_db);
        self.update_headroom();
    }

    pub fn set_track_metadata(&mut self, meta: &ReplayGainMetadata) {
        self.replay_gain.set_track_metadata(meta);
    }

    pub fn set_dither_mode(&mut self, mode: DitherMode) {
        self.dither.set_mode(mode);
    }

    /// Automatically compute pre-cut gain headroom to prevent clipping when EQ bands are boosted.
    fn update_headroom(&mut self) {
        let mut max_boost_db = 0.0f32;
        for config in self.eq_left.configs() {
            if config.enabled && config.gain_db > 0.0 {
                max_boost_db = max_boost_db.max(config.gain_db);
            }
        }
        let pre_cut_db = if max_boost_db > 0.0 {
            -(max_boost_db + 1.0)
        } else {
            0.0
        };
        self.headroom_gain = db_to_linear(pre_cut_db);
    }

    /// Process a single stereo sample frame through the entire configured DSP pipeline.
    #[inline(always)]
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        // Strict Bit-Perfect Bypass: 100% pass-through
        if self.mode == PlaybackMode::BitPerfect {
            return (left, right);
        }

        // STAGE 4: ReplayGain
        let (rg_l, rg_r) = self.replay_gain.process_stereo(left, right);

        // STAGE 5: Headroom + Equalizer
        let hr_l = rg_l * self.headroom_gain;
        let hr_r = rg_r * self.headroom_gain;
        let eq_l = self.eq_left.process_sample(hr_l);
        let eq_r = self.eq_right.process_sample(hr_r);

        // STAGE 7: Master Volume Fader
        let vol = self.master_gain.process_sample(1.0);
        let vol_l = eq_l * vol;
        let vol_r = eq_r * vol;

        // STAGE 7: True-Peak Lookahead Limiter
        let (lim_l, lim_r) = self.limiter.process_stereo(vol_l, vol_r);

        // STAGE 8: TPDF Dither
        self.dither.process_stereo(lim_l, lim_r)
    }

    /// Process an interleaved stereo slice in-place.
    pub fn process_interleaved(&mut self, buffer: &mut [f32]) {
        if self.mode == PlaybackMode::BitPerfect {
            return;
        }

        for chunk in buffer.chunks_exact_mut(2) {
            let (l, r) = self.process_stereo(chunk[0], chunk[1]);
            chunk[0] = l;
            chunk[1] = r;
        }
    }

    pub fn reset(&mut self) {
        self.replay_gain.reset();
        self.eq_left.reset();
        self.eq_right.reset();
        self.limiter.reset();
        self.dither.reset();
        self.master_gain = Gain::new(if self.mode == PlaybackMode::BitPerfect {
            1.0
        } else {
            self.volume
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_perfect_mode_exact_identity() {
        let mut pipeline = DspPipeline::new(48000.0);
        pipeline.set_mode(PlaybackMode::BitPerfect);
        pipeline.set_volume(0.5); // Should be ignored in bit-perfect mode

        let test_samples = [0.123456f32, -0.987654f32, 0.0f32, 1.0f32, -1.0f32];
        for &s in &test_samples {
            let (l, r) = pipeline.process_stereo(s, s);
            assert_eq!(l, s, "Bit-perfect left channel altered");
            assert_eq!(r, s, "Bit-perfect right channel altered");
        }
    }
}
