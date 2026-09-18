//! Sonora Spectrum+ visualizer — pure DSP logic (host-independent).
//!
//! The plugin reads FFT magnitude bins through the read-only `tap_read` host
//! function and computes three render modes. Rendering itself stays
//! client-side; `visualizer_info` returns the descriptor plus the latest
//! computed frame for all modes, so any client can render without extra
//! round-trips. Everything here is unit-testable on the host.

use serde::{Deserialize, Serialize};

/// Output bars per mode.
pub const OUT_BINS: usize = 64;
/// Maximum tap floats consumed per call (host serves up to 256).
pub const MAX_TAP: usize = 256;
/// Release smoothing factor applied to decaying bars (attack is instant).
pub const RELEASE: f32 = 0.82;

/// Render modes computed per call.
pub const MODES: [&str; 3] = ["bars", "wave", "mirror"];

/// Descriptor + latest frame, as returned by `visualizer_info`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizerFrame {
    pub name: String,
    pub modes: Vec<String>,
    pub preferred_fps: u32,
    pub uses_audio_tap: bool,
    pub frame: FrameData,
}

/// Computed bars for every mode (each `OUT_BINS` values in 0.0..=1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameData {
    pub bins: usize,
    pub bars: Vec<f32>,
    pub wave: Vec<f32>,
    pub mirror: Vec<f32>,
}

fn sanitize(v: f32) -> f32 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        0.0
    }
}

/// Group up to 256 tap bins into 64 bars, peak-normalize, sqrt-compress.
/// Deterministic, allocation-free apart from the fixed output, O(n).
pub fn compute_bars(frame: &[f32]) -> [f32; OUT_BINS] {
    let mut bars = [0.0f32; OUT_BINS];
    let n = frame.len().min(MAX_TAP);
    if n == 0 {
        return bars;
    }
    let mut peak = 0.0f32;
    for i in 0..OUT_BINS {
        let start = i * n / OUT_BINS;
        let end = ((i + 1) * n / OUT_BINS).max(start + 1).min(n);
        let mut sum = 0.0f32;
        let mut count = 0u32;
        for &v in &frame[start..end] {
            sum += sanitize(v);
            count += 1;
        }
        let avg = if count > 0 { sum / count as f32 } else { 0.0 };
        bars[i] = avg;
        if avg > peak {
            peak = avg;
        }
    }
    if peak > 0.0 {
        for b in bars.iter_mut() {
            *b = (*b / peak).sqrt().min(1.0);
        }
    }
    bars
}

/// Wave mode: linearly normalized downsample of the raw bins.
pub fn compute_wave(frame: &[f32]) -> [f32; OUT_BINS] {
    let mut wave = [0.0f32; OUT_BINS];
    let n = frame.len().min(MAX_TAP);
    if n == 0 {
        return wave;
    }
    let mut peak = 0.0f32;
    for i in 0..OUT_BINS {
        let start = i * n / OUT_BINS;
        let end = ((i + 1) * n / OUT_BINS).max(start + 1).min(n);
        let mut sum = 0.0f32;
        let mut count = 0u32;
        for &v in &frame[start..end] {
            sum += sanitize(v);
            count += 1;
        }
        let avg = if count > 0 { sum / count as f32 } else { 0.0 };
        wave[i] = avg;
        if avg > peak {
            peak = avg;
        }
    }
    if peak > 0.0 {
        for w in wave.iter_mut() {
            *w = (*w / peak).min(1.0);
        }
    }
    wave
}

/// Mirror mode: symmetric fold that preserves peaks —
/// `out[i] = max(bars[i], bars[63-i])`, so `out[i] == out[63-i]` always holds
/// and no strong bin is ever dropped.
pub fn compute_mirror(bars: &[f32; OUT_BINS]) -> [f32; OUT_BINS] {
    let mut out = [0.0f32; OUT_BINS];
    for i in 0..OUT_BINS {
        out[i] = bars[i].max(bars[OUT_BINS - 1 - i]);
    }
    out
}

/// Release smoothing: instant attack, exponential decay. Keeps motion fluid
/// at 60 fps without per-frame allocation on the caller side.
pub fn smooth(prev: &[f32; OUT_BINS], cur: &[f32; OUT_BINS]) -> [f32; OUT_BINS] {
    let mut out = [0.0f32; OUT_BINS];
    for i in 0..OUT_BINS {
        out[i] = if cur[i] >= prev[i] {
            cur[i]
        } else {
            (prev[i] * RELEASE).min(1.0)
        };
    }
    out
}

pub fn descriptor_json(frame: &FrameData) -> String {
    serde_json::to_string(&VisualizerFrame {
        name: "Spectrum+".to_string(),
        modes: MODES.iter().map(|s| s.to_string()).collect(),
        preferred_fps: 60,
        uses_audio_tap: true,
        frame: frame.clone(),
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spike_bin(bin: usize, n: usize) -> Vec<f32> {
        let mut v = vec![0.0f32; n];
        v[bin] = 1.0;
        v
    }

    #[test]
    fn empty_frame_yields_zeros() {
        assert_eq!(compute_bars(&[]), [0.0; OUT_BINS]);
        assert_eq!(compute_wave(&[]), [0.0; OUT_BINS]);
        assert_eq!(compute_mirror(&[0.0; OUT_BINS]), [0.0; OUT_BINS]);
    }

    #[test]
    fn spike_maps_to_expected_bar_with_peak_normalization() {
        // 256 bins -> bar i covers bins [4i, 4i+4); spike at bin 130 -> bar 32.
        let bars = compute_bars(&spike_bin(130, 256));
        assert_eq!(bars.len(), OUT_BINS);
        assert!((bars[32] - 1.0).abs() < 1e-6);
        assert!(bars.iter().enumerate().all(|(i, &b)| i == 32 || b == 0.0));
        assert!(bars.iter().all(|&b| (0.0..=1.0).contains(&b)));
    }

    #[test]
    fn short_frames_still_cover_all_bars() {
        let frame: Vec<f32> = (0..16).map(|i| i as f32 / 16.0).collect();
        let bars = compute_bars(&frame);
        assert!(bars.iter().all(|&b| (0.0..=1.0).contains(&b)));
        assert!(bars[63] > bars[0]);
    }

    #[test]
    fn non_finite_input_cannot_escape() {
        let frame = vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -3.0];
        let bars = compute_bars(&frame);
        let wave = compute_wave(&frame);
        assert!(bars.iter().all(|&b| b == 0.0));
        assert!(wave.iter().all(|&b| b == 0.0));
    }

    #[test]
    fn mirror_is_symmetric_and_preserves_peaks() {
        let mut bars = [0.0f32; OUT_BINS];
        for (i, b) in bars.iter_mut().enumerate() {
            *b = i as f32 / OUT_BINS as f32;
        }
        bars[32] = 1.0;
        let m = compute_mirror(&bars);
        for i in 0..OUT_BINS {
            assert!((m[i] - m[OUT_BINS - 1 - i]).abs() < 1e-6);
        }
        assert!((m[31] - 1.0).abs() < 1e-6);
        assert!((m[32] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn smoothing_attacks_instantly_and_releases() {
        let prev = [0.8f32; OUT_BINS];
        let loud = [1.0f32; OUT_BINS];
        let silent = [0.0f32; OUT_BINS];
        assert_eq!(smooth(&prev, &loud), loud);
        let decayed = smooth(&prev, &silent);
        assert!((decayed[0] - 0.8 * RELEASE).abs() < 1e-6);
        assert!(decayed.iter().all(|&b| b < 0.8 && b >= 0.0));
    }

    #[test]
    fn descriptor_json_shape() {
        let frame = FrameData {
            bins: 0,
            bars: vec![0.0; OUT_BINS],
            wave: vec![0.0; OUT_BINS],
            mirror: vec![0.0; OUT_BINS],
        };
        let parsed: VisualizerFrame = serde_json::from_str(&descriptor_json(&frame)).unwrap();
        assert_eq!(parsed.name, "Spectrum+");
        assert_eq!(parsed.modes, vec!["bars", "wave", "mirror"]);
        assert_eq!(parsed.preferred_fps, 60);
        assert!(parsed.uses_audio_tap);
    }
}
