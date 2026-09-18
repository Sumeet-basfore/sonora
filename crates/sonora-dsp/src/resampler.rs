/// Simple real-time linear resampler for converting stereo audio frames
/// between arbitrary sample rates (e.g. 44100 Hz to 48000 Hz).
/// Continuous phase and inter-buffer boundary interpolation with zero heap allocations.
pub struct LinearResampler {
    source_rate: f32,
    target_rate: f32,
    ratio: f32,
    phase: f32,
    last_left: f32,
    last_right: f32,
    has_last_sample: bool,
}

impl LinearResampler {
    pub fn new(source_rate: f32, target_rate: f32) -> Self {
        let ratio = if target_rate > 0.0 {
            source_rate / target_rate
        } else {
            1.0
        };
        Self {
            source_rate,
            target_rate,
            ratio,
            phase: 0.0,
            last_left: 0.0,
            last_right: 0.0,
            has_last_sample: false,
        }
    }

    pub fn set_rates(&mut self, source_rate: f32, target_rate: f32) {
        self.source_rate = source_rate;
        self.target_rate = target_rate;
        self.ratio = if target_rate > 0.0 {
            source_rate / target_rate
        } else {
            1.0
        };
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.last_left = 0.0;
        self.last_right = 0.0;
        self.has_last_sample = false;
    }

    /// Resample interleaved stereo input slice into output slice.
    /// Returns the number of stereo frames written to `output`.
    pub fn resample_stereo(&mut self, input: &[f32], output: &mut [f32]) -> usize {
        if (self.source_rate - self.target_rate).abs() < 1.0 {
            // Passthrough when rates match
            let to_copy = input.len().min(output.len());
            output[..to_copy].copy_from_slice(&input[..to_copy]);
            return to_copy / 2;
        }

        let input_frames = input.len() / 2;
        let output_frames = output.len() / 2;
        if input_frames == 0 || output_frames == 0 {
            return 0;
        }

        if !self.has_last_sample {
            self.last_left = input[0];
            self.last_right = input[1];
            self.has_last_sample = true;
            self.phase = 0.0;
        }

        let mut out_frame = 0;
        while out_frame < output_frames {
            if self.phase < 0.0 {
                // Interpolate between previous buffer's last frame and current buffer's frame 0
                let frac = self.phase + 1.0;
                let next_l = input[0];
                let next_r = input[1];

                let out_l = self.last_left + frac * (next_l - self.last_left);
                let out_r = self.last_right + frac * (next_r - self.last_right);

                output[out_frame * 2] = out_l;
                output[out_frame * 2 + 1] = out_r;

                out_frame += 1;
                self.phase += self.ratio;
            } else {
                let in_idx = self.phase.floor() as usize;
                if in_idx + 1 >= input_frames {
                    // Requires next frame which is in subsequent buffer
                    break;
                }

                let frac = self.phase - in_idx as f32;
                let curr_l = input[in_idx * 2];
                let curr_r = input[in_idx * 2 + 1];
                let next_l = input[(in_idx + 1) * 2];
                let next_r = input[(in_idx + 1) * 2 + 1];

                let out_l = curr_l + frac * (next_l - curr_l);
                let out_r = curr_r + frac * (next_r - curr_r);

                output[out_frame * 2] = out_l;
                output[out_frame * 2 + 1] = out_r;

                out_frame += 1;
                self.phase += self.ratio;
            }
        }

        // Store last frame of input buffer for next chunk
        self.last_left = input[(input_frames - 1) * 2];
        self.last_right = input[(input_frames - 1) * 2 + 1];

        // Adjust phase relative to consumed input frames
        self.phase -= (input_frames - 1) as f32 + 1.0;

        out_frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_continuity_across_chunks() {
        let mut resampler = LinearResampler::new(44100.0, 48000.0);
        let num_total_frames = 4096;
        let mut full_input = Vec::with_capacity(num_total_frames * 2);
        for i in 0..num_total_frames {
            let s = ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / 44100.0).sin();
            full_input.push(s);
            full_input.push(s);
        }

        // Resample in small chunks of 256 frames
        let mut chunked_output = Vec::new();
        let chunk_size_frames = 256;
        let mut temp_out = vec![0.0f32; 1024];

        for chunk in full_input.chunks(chunk_size_frames * 2) {
            let written = resampler.resample_stereo(chunk, &mut temp_out);
            chunked_output.extend_from_slice(&temp_out[..written * 2]);
        }

        assert!(chunked_output.len() > 1000);
        // Verify no NaNs or abrupt discontinuities
        for window in chunked_output.windows(4) {
            let diff_left = (window[2] - window[0]).abs();
            assert!(
                diff_left < 0.25,
                "Found discontinuity in resampled audio: {diff_left}"
            );
        }
    }
}
