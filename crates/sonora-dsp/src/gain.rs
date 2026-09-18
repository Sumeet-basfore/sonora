/// Convert decibels to linear amplitude scalar.
#[inline(always)]
pub fn db_to_linear(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

/// Convert linear amplitude scalar to decibels.
#[inline(always)]
pub fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        -100.0
    } else {
        20.0 * linear.log10()
    }
}

/// Real-time safe gain processor with smooth interpolation.
#[derive(Debug, Clone, Copy)]
pub struct Gain {
    current: f32,
    target: f32,
    smoothing_factor: f32,
}

impl Default for Gain {
    fn default() -> Self {
        Self {
            current: 1.0,
            target: 1.0,
            smoothing_factor: 0.005,
        }
    }
}

impl Gain {
    pub fn new(initial_gain: f32) -> Self {
        Self {
            current: initial_gain,
            target: initial_gain,
            smoothing_factor: 0.005,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    #[inline(always)]
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        self.current += (self.target - self.current) * self.smoothing_factor;
        sample * self.current
    }
}
