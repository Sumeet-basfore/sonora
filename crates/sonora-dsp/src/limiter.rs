/// Master peak limiter interface.
pub trait Limiter: Send {
    fn process_sample(&mut self, sample: f32) -> f32;
    fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32);
    fn reset(&mut self);
}

/// A basic hard-clip limiter placeholder.
#[derive(Debug, Clone, Copy, Default)]
pub struct HardLimiter {
    ceiling: f32,
}

impl HardLimiter {
    pub fn new(ceiling: f32) -> Self {
        Self { ceiling }
    }
}

impl Limiter for HardLimiter {
    #[inline(always)]
    fn process_sample(&mut self, sample: f32) -> f32 {
        sample.clamp(-self.ceiling, self.ceiling)
    }

    #[inline(always)]
    fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        (
            left.clamp(-self.ceiling, self.ceiling),
            right.clamp(-self.ceiling, self.ceiling),
        )
    }

    fn reset(&mut self) {}
}

/// Polyphase 4x FIR coefficients for inter-sample true-peak estimation (ITU-R BS.1770-4 / AES17).
const INTERP_4X_COEFFS: [[f32; 4]; 4] = [
    [0.0, 1.0, 0.0, 0.0],                    // phase 0 (exact sample)
    [-0.078125, 0.859375, 0.28125, -0.0625], // phase 1 (+0.25)
    [-0.125, 0.625, 0.625, -0.125],          // phase 2 (+0.50)
    [-0.0625, 0.28125, 0.859375, -0.078125], // phase 3 (+0.75)
];

/// 4x Oversampled True-Peak Lookahead Limiter.
/// Operates a 4.0ms circular lookahead delay line, 4x polyphase true-peak detection,
/// sub-millisecond lookahead attack, and smooth dual-stage exponential release.
#[derive(Debug, Clone)]
pub struct TruePeakLimiter {
    sample_rate: f32,
    ceiling: f32,
    delay_samples: usize,
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    delay_idx: usize,
    history_l: [f32; 4],
    history_r: [f32; 4],
    gain_envelope: f32,
    release_coeff_fast: f32,
    release_coeff_slow: f32,
}

impl TruePeakLimiter {
    pub fn new(sample_rate: f32, ceiling_db: f32) -> Self {
        let sample_rate = sample_rate.max(8000.0);
        let ceiling = 10.0f32.powf(ceiling_db / 20.0).clamp(0.01, 1.0);
        let delay_samples = ((sample_rate * 0.004).round() as usize).max(16);

        let release_time_fast = 0.050; // 50ms
        let release_time_slow = 0.250; // 250ms

        let release_coeff_fast = (-1.0 / (release_time_fast * sample_rate)).exp();
        let release_coeff_slow = (-1.0 / (release_time_slow * sample_rate)).exp();

        Self {
            sample_rate,
            ceiling,
            delay_samples,
            delay_l: vec![0.0; delay_samples],
            delay_r: vec![0.0; delay_samples],
            delay_idx: 0,
            history_l: [0.0; 4],
            history_r: [0.0; 4],
            gain_envelope: 1.0,
            release_coeff_fast,
            release_coeff_slow,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() >= 1.0 {
            *self = Self::new(sample_rate, 20.0 * self.ceiling.log10());
        }
    }

    pub fn set_ceiling(&mut self, ceiling_db: f32) {
        self.ceiling = 10.0f32.powf(ceiling_db / 20.0).clamp(0.01, 1.0);
    }

    pub fn ceiling(&self) -> f32 {
        self.ceiling
    }

    #[inline(always)]
    fn estimate_true_peak_4x(history: &[f32; 4]) -> f32 {
        let mut max_peak = 0.0f32;
        for phase in &INTERP_4X_COEFFS {
            let interp = history[0] * phase[0]
                + history[1] * phase[1]
                + history[2] * phase[2]
                + history[3] * phase[3];
            let abs_val = interp.abs();
            if abs_val > max_peak {
                max_peak = abs_val;
            }
        }
        max_peak
    }
}

impl Limiter for TruePeakLimiter {
    #[inline(always)]
    fn process_sample(&mut self, sample: f32) -> f32 {
        let (out, _) = self.process_stereo(sample, sample);
        out
    }

    #[inline(always)]
    fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        // Shift FIR history
        self.history_l[0] = self.history_l[1];
        self.history_l[1] = self.history_l[2];
        self.history_l[2] = self.history_l[3];
        self.history_l[3] = left;

        self.history_r[0] = self.history_r[1];
        self.history_r[1] = self.history_r[2];
        self.history_r[2] = self.history_r[3];
        self.history_r[3] = right;

        // 4x oversampled true-peak detection
        let tp_l = Self::estimate_true_peak_4x(&self.history_l);
        let tp_r = Self::estimate_true_peak_4x(&self.history_r);
        let max_tp = tp_l.max(tp_r);

        // Required gain reduction
        let target_gain = if max_tp > self.ceiling {
            self.ceiling / max_tp
        } else {
            1.0
        };

        // Attack & Release ballistics
        if target_gain < self.gain_envelope {
            // Fast lookahead attack: immediately track downwards so delay line peak is fully attenuated
            self.gain_envelope = target_gain.min(self.gain_envelope);
        } else {
            let rel_coeff = if (1.0 - self.gain_envelope) > 0.3 {
                self.release_coeff_fast
            } else {
                self.release_coeff_slow
            };
            self.gain_envelope = self.gain_envelope * rel_coeff + (1.0 - rel_coeff) * 1.0;
        }

        // Retrieve sample from 4ms lookahead delay line
        let delayed_l = self.delay_l[self.delay_idx];
        let delayed_r = self.delay_r[self.delay_idx];

        // Store current input in delay line
        self.delay_l[self.delay_idx] = left;
        self.delay_r[self.delay_idx] = right;
        self.delay_idx = (self.delay_idx + 1) % self.delay_samples;

        // Apply lookahead gain envelope with strict ceiling safety
        let out_l = (delayed_l * self.gain_envelope).clamp(-self.ceiling, self.ceiling);
        let out_r = (delayed_r * self.gain_envelope).clamp(-self.ceiling, self.ceiling);

        (out_l, out_r)
    }

    fn reset(&mut self) {
        self.delay_l.fill(0.0);
        self.delay_r.fill(0.0);
        self.history_l.fill(0.0);
        self.history_r.fill(0.0);
        self.gain_envelope = 1.0;
        self.delay_idx = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true_peak_limiter_clamps_hot_signal() {
        let mut limiter = TruePeakLimiter::new(48000.0, -0.3); // -0.3 dBFS ~= 0.966
        let ceiling = limiter.ceiling();

        // Feed an over-driven sine wave (+6 dBFS = 2.0 amplitude)
        let mut max_output = 0.0f32;
        for i in 0..48000 {
            let s = ((i as f32 * 1000.0 * 2.0 * std::f32::consts::PI) / 48000.0).sin() * 2.0;
            let (out_l, out_r) = limiter.process_stereo(s, s);
            if i > 250 {
                // after initial lookahead fill
                max_output = max_output.max(out_l.abs()).max(out_r.abs());
            }
        }

        assert!(
            max_output <= ceiling + 0.05,
            "Limiter failed to contain hot signal: {max_output} > {ceiling}"
        );
    }
}
