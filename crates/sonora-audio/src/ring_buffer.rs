use rtrb::{Consumer, Producer, RingBuffer};

/// Lock-free Single-Producer Single-Consumer (SPSC) ring buffer for real-time audio frames.
/// Zero allocation, wait-free, and lock-free across thread boundaries.
pub struct AudioRingBuffer;

impl AudioRingBuffer {
    /// Create a new SPSC ring buffer for audio samples with the given capacity (number of f32 samples).
    pub fn create(capacity: usize) -> (Producer<f32>, Consumer<f32>) {
        RingBuffer::new(capacity)
    }
}
