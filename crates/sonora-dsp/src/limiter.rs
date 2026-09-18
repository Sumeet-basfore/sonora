/// Master peak limiter interface.
pub trait Limiter: Send {
    fn process_sample(&mut self, sample: f32) -> f32;
    fn reset(&mut self);
}

/// A basic hard-clip limiter placeholder for bootstrap.
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

    fn reset(&mut self) {}
}
