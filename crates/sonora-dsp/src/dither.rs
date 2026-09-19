use serde::{Deserialize, Serialize};

/// Dithering bit-depth target format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DitherMode {
    #[default]
    None,
    Tpdf16,
    Tpdf24,
}

/// Triangular Probability Density Function (TPDF) dither generator.
/// Prevents quantization distortion and harmonic truncation artifacts when formatting
/// 32-bit floating point audio for 16-bit or 24-bit DAC word lengths.
#[derive(Debug, Clone)]
pub struct TpdfDither {
    mode: DitherMode,
    rng_state: u64,
}

impl Default for TpdfDither {
    fn default() -> Self {
        Self::new(DitherMode::None)
    }
}

impl TpdfDither {
    pub fn new(mode: DitherMode) -> Self {
        Self {
            mode,
            rng_state: 0x853c49e6748fea9b,
        }
    }

    pub fn set_mode(&mut self, mode: DitherMode) {
        self.mode = mode;
    }

    pub fn mode(&self) -> DitherMode {
        self.mode
    }

    #[inline(always)]
    fn next_uniform(&mut self) -> f32 {
        // Fast Xorshift64 PRNG
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        let val = (self.rng_state & 0x00ffffff) as f32; // 24-bit precision
        (val / 8388608.0) - 1.0 // Range [-1.0, 1.0]
    }

    #[inline(always)]
    fn next_tpdf(&mut self) -> f32 {
        let r1 = self.next_uniform() * 0.5;
        let r2 = self.next_uniform() * 0.5;
        r1 + r2 // Triangular distribution in [-1.0, 1.0]
    }

    #[inline(always)]
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        match self.mode {
            DitherMode::None => sample,
            DitherMode::Tpdf16 => {
                let scale = 32767.0f32;
                let scaled = sample * scale;
                let dither = self.next_tpdf();
                let quantized = (scaled + dither).round();
                (quantized / scale).clamp(-1.0, 1.0)
            }
            DitherMode::Tpdf24 => {
                let scale = 8388607.0f32;
                let scaled = sample * scale;
                let dither = self.next_tpdf();
                let quantized = (scaled + dither).round();
                (quantized / scale).clamp(-1.0, 1.0)
            }
        }
    }

    #[inline(always)]
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        (self.process_sample(left), self.process_sample(right))
    }

    pub fn reset(&mut self) {
        self.rng_state = 0x853c49e6748fea9b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tpdf_dither_preserves_bounds() {
        let mut dither = TpdfDither::new(DitherMode::Tpdf16);
        for i in 0..1000 {
            let s = (i as f32 / 500.0) - 1.0;
            let out = dither.process_sample(s);
            assert!((-1.0..=1.0).contains(&out));
        }
    }
}
