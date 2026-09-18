/// Simple real-time linear resampler for converting stereo audio frames
/// between arbitrary sample rates (e.g. 44100 Hz to 48000 Hz).
/// Deterministic and zero heap allocations in process_frames.
pub struct LinearResampler {
    source_rate: f32,
    target_rate: f32,
    ratio: f32,
    phase: f32,
    last_left: f32,
    last_right: f32,
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

        let mut out_frame = 0;
        while out_frame < output_frames {
            let in_idx = self.phase.floor() as usize;
            let frac = self.phase - in_idx as f32;

            let (curr_l, curr_r) = if in_idx < input_frames {
                (input[in_idx * 2], input[in_idx * 2 + 1])
            } else {
                break;
            };

            let (next_l, next_r) = if in_idx + 1 < input_frames {
                (input[(in_idx + 1) * 2], input[(in_idx + 1) * 2 + 1])
            } else {
                (curr_l, curr_r)
            };

            let out_l = curr_l + frac * (next_l - curr_l);
            let out_r = curr_r + frac * (next_r - curr_r);

            output[out_frame * 2] = out_l;
            output[out_frame * 2 + 1] = out_r;

            out_frame += 1;
            self.phase += self.ratio;
        }

        // Adjust phase relative to consumed input frames
        self.phase -= input_frames as f32;
        if self.phase < 0.0 {
            self.phase = 0.0;
        }

        out_frame
    }
}
