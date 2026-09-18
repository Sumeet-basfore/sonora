use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::f32::consts::PI;
use std::sync::Arc;

pub const DEFAULT_FFT_SIZE: usize = 1024;
pub const DEFAULT_SPECTRUM_BANDS: usize = 48;

/// FFT-based audio spectrum analyzer for real-time frequency visualization.
/// Operates outside the real-time audio thread.
pub struct SpectrumAnalyzer {
    fft_size: usize,
    num_bands: usize,
    sample_rate: f32,
    window: Vec<f32>,
    fft: Arc<dyn Fft<f32>>,
    complex_buffer: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    band_bin_ranges: Vec<(usize, usize)>,
}

impl SpectrumAnalyzer {
    pub fn new(fft_size: usize, num_bands: usize, sample_rate: f32) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let scratch_len = fft.get_inplace_scratch_len();

        // Precompute Hann window
        let window: Vec<f32> = (0..fft_size)
            .map(|n| 0.5 * (1.0 - (2.0 * PI * n as f32 / (fft_size - 1) as f32).cos()))
            .collect();

        let mut analyzer = Self {
            fft_size,
            num_bands,
            sample_rate,
            window,
            fft,
            complex_buffer: vec![Complex::new(0.0, 0.0); fft_size],
            scratch: vec![Complex::new(0.0, 0.0); scratch_len],
            band_bin_ranges: Vec::with_capacity(num_bands),
        };

        analyzer.compute_band_ranges();
        analyzer
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() > 1.0 {
            self.sample_rate = sample_rate;
            self.compute_band_ranges();
        }
    }

    fn compute_band_ranges(&mut self) {
        self.band_bin_ranges.clear();
        let min_freq = 25.0f32;
        let max_freq = (self.sample_rate * 0.5).min(20000.0);
        let num_bands = self.num_bands;
        let half_fft = self.fft_size / 2;

        let freq_to_bin = |f: f32| -> usize {
            let bin = (f * self.fft_size as f32 / self.sample_rate).round() as usize;
            bin.clamp(0, half_fft.saturating_sub(1))
        };

        for i in 0..num_bands {
            let f_low = min_freq * (max_freq / min_freq).powf(i as f32 / num_bands as f32);
            let f_high = min_freq * (max_freq / min_freq).powf((i + 1) as f32 / num_bands as f32);

            let mut b_start = freq_to_bin(f_low);
            let mut b_end = freq_to_bin(f_high);
            if b_end <= b_start {
                b_end = b_start + 1;
            }
            if b_start >= half_fft {
                b_start = half_fft.saturating_sub(1);
            }
            if b_end > half_fft {
                b_end = half_fft;
            }
            self.band_bin_ranges.push((b_start, b_end));
        }
    }

    /// Compute logarithmic spectrum bands normalized to [0.0, 1.0] from raw time-domain audio samples.
    pub fn compute_spectrum(&mut self, samples: &[f32]) -> Vec<f32> {
        let n = self.fft_size;
        let sample_count = samples.len();

        // Window the incoming samples into complex_buffer
        for (slot, (&sample, &w)) in self
            .complex_buffer
            .iter_mut()
            .zip(samples.iter().zip(&self.window))
        {
            *slot = Complex::new(sample * w, 0.0);
        }
        if sample_count < n {
            for slot in &mut self.complex_buffer[sample_count..n] {
                *slot = Complex::new(0.0, 0.0);
            }
        }

        // Perform in-place forward FFT
        self.fft
            .process_with_scratch(&mut self.complex_buffer, &mut self.scratch);

        // Compute magnitude and group into frequency bands
        let half_n = n / 2;
        let scale = 2.0 / n as f32;
        let min_db = -65.0f32;
        let max_db = 0.0f32;

        let mut bands = Vec::with_capacity(self.num_bands);

        for (band_idx, &(b_start, b_end)) in self.band_bin_ranges.iter().enumerate() {
            let mut peak_mag = 0.0f32;
            for bin in b_start..b_end.min(half_n) {
                let c = self.complex_buffer[bin];
                let mag = (c.re * c.re + c.im * c.im).sqrt() * scale;
                if mag > peak_mag {
                    peak_mag = mag;
                }
            }

            // Apply slight high-frequency equalization boost (tilt) for visual balance
            let tilt_factor = 1.0 + (band_idx as f32 / self.num_bands as f32) * 1.5;
            let boosted_mag = peak_mag * tilt_factor;

            let db = if boosted_mag > 1e-6 {
                20.0 * boosted_mag.log10()
            } else {
                min_db
            };

            let norm = ((db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
            bands.push(norm);
        }

        bands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectrum_analyzer_silence() {
        let mut analyzer = SpectrumAnalyzer::new(1024, 48, 48000.0);
        let silence = vec![0.0f32; 1024];
        let spectrum = analyzer.compute_spectrum(&silence);
        assert_eq!(spectrum.len(), 48);
        for &val in &spectrum {
            assert_eq!(val, 0.0);
        }
    }

    #[test]
    fn test_spectrum_analyzer_sine_wave() {
        let mut analyzer = SpectrumAnalyzer::new(1024, 48, 48000.0);
        let freq = 1000.0f32;
        let mut sine = vec![0.0f32; 1024];
        for (i, sample) in sine.iter_mut().enumerate() {
            *sample = (2.0 * PI * freq * i as f32 / 48000.0).sin();
        }

        let spectrum = analyzer.compute_spectrum(&sine);
        assert_eq!(spectrum.len(), 48);
        let max_val = spectrum.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            max_val > 0.7,
            "Expected high response around 1kHz peak, got {max_val}"
        );
    }
}
