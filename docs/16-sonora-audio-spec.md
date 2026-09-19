# Sonora Audio Engine Architecture & Playback Pipeline Specification

## 1. Executive Summary & Engine Mandate

The Sonora Audio Engine is engineered from the ground up to deliver uncompromising audiophile fidelity, sample-accurate gapless playback, sub-millisecond responsiveness, and robust cross-platform output hardware control. This specification defines the exact signal processing pipeline, output modes, device management, DSP ordering, and automated verification methodologies.

---

## 2. The Complete Sonora Audio Playback Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     SONORA COMPLETE AUDIO SIGNAL PIPELINE                   │
│                                                                             │
│  [Audio Source: Local FLAC / MP3 / WAV / ALAC / Network Stream]             │
│                              │                                              │
│                              ▼                                              │
│  [STAGE 1: Decode & Header Parsing]                                         │
│  • Pure-Rust Symphonia Decoder                                              │
│  • Strip Encoder Delay & End Padding (LAME / iTunSMPB tags)                 │
│  • Dual-Decoder Asynchronous Pre-buffering for Gapless Transitions          │
│                              │                                              │
│                              ▼ [Interleaved PCM Float32/Float64 Samples]    │
│  [STAGE 2: Channel Handling & Matrixing]                                    │
│  • Mono Upmixing / Stereo Passthrough / Multi-channel Downmix               │
│  • Channel Remapping & Polarity Inversion                                   │
│                              │                                              │
│                              ▼                                              │
│  [STAGE 3: Sample-Rate Handling]                                            │
│  • Bit-Perfect Path: Rate Match Hardware & Passthrough                      │
│  • Resampling Path: Polyphase Sinc Resampling via `rubato` (SIMD AVX2/NEON) │
│                              │                                              │
│                              ▼                                              │
│  [STAGE 4: ReplayGain / EBU R128 Loudness Normalization]                    │
│  • Track Gain / Album Gain selection                                        │
│  • Target Loudness Reference (-18.0 LUFS or -14.0 LUFS)                     │
│  • Dynamic Pre-Amp Attenuation (Peak Clipping Prevention)                   │
│                              │                                              │
│                              ▼                                              │
│  [STAGE 5: Equalizer (EQ) Stage]                                            │
│  • 10+ Band Parametric Biquad Filters (Direct Form II Transposed)           │
│  • AutoEQ Target Curve Integration                                          │
│  • Automatic Pre-Cut Gain Headroom Compensation                             │
│  • Denormal Float Elimination (`_MM_SET_FLUSH_ZERO_MODE`)                   │
│                              │                                              │
│                              ▼                                              │
│  [STAGE 6: Secondary DSP & Spatial Processing]                              │
│  • Bauer Binaural Crossfeed (Headphone fatigue reduction)                   │
│  • Tube / Harmonic Warmth Saturation (Optional)                             │
│  • User Plugin DSP Graph Node Hooks                                         │
│                              │                                              │
│                              ▼                                              │
│  [STAGE 7: Master Volume & True Peak Limiter]                               │
│  • Logarithmic 64-bit Master Gain Fader                                     │
│  • 4x Oversampled True-Peak Lookahead Limiter                               │
│  • Real-Time Visualizer Audio Tap (Post-Limiter Shared Ring Buffer)         │
│                              │                                              │
│                              ▼                                              │
│  [STAGE 8: Dither & Bit-Depth Quantization]                                 │
│  • TPDF (Triangular Probability Density Function) Dither                    │
│  • 16-bit / 24-bit / 32-bit Integer / Float32 Formatting                    │
│                              │                                              │
│                              ▼                                              │
│  [STAGE 9: Platform Audio Output Driver]                                    │
│  • Windows: WASAPI Exclusive Event Mode / Shared CPAL Stream                │
│  • macOS: CoreAudio Hog Mode / AudioUnit Output                             │
│  • Linux: ALSA Direct Hardware (`hw:`) / PipeWire Native Audio Stream       │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. The Four Core Playback Modes

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SONORA PLAYBACK MODES                             │
├───────────────────┬─────────────┬──────────────┬──────────────┬─────────────┤
│ Mode              │ Resampling  │ DSP / EQ     │ Output Type  │ Latency     │
├───────────────────┼─────────────┼──────────────┼──────────────┼─────────────┤
│ 1. Default Shared │ Rubato Sinc │ Fully Active │ OS Shared    │ 20ms - 40ms │
│ 2. High-Quality   │ Rubato Sinc │ Active + Pre │ OS Shared/Ex │ 10ms - 20ms │
│ 3. Bit-Perfect    │ None (Byp)  │ 100% Bypass  │ Exclusive    │ 2ms - 5ms   │
│ 4. Exclusive DSP  │ Auto-Switch │ Active + Lim │ Exclusive    │ 2ms - 8ms   │
└───────────────────┴─────────────┴──────────────┴──────────────┴─────────────┘
```

### 3.1 Mode 1: Default Playback Mode (Universal Shared)
- **Target Use Case**: Daily casual listening while sharing audio with web browsers, Discord, and system notifications.
- **Engine Behavior**:
  - Operates via standard CPAL / OS Shared mixer.
  - Automatically resamples non-native audio to match the OS mixer rate (e.g. 44.1 kHz -> 48.0 kHz) using `rubato` high-quality Sinc interpolation.
  - ReplayGain 2.0, Parametric EQ, and True-Peak Safety Limiter are active.

### 3.2 Mode 2: High-Quality Mode (Audiophile Shared)
- **Target Use Case**: Audiophiles wanting EQ and headphone correction while maintaining system sound compatibility.
- **Engine Behavior**:
  - Operates internal audio pipeline in full **64-bit floating point (`f64`)**.
  - Dynamic sample rate negotiation with PipeWire (Linux) or OS mixer where supported.
  - 4x oversampled True-Peak lookahead limiter prevents inter-sample clipping on DAC reconstruction.

### 3.3 Mode 3: Bit-Perfect Mode (Pure Hardware Passthrough)
- **Target Use Case**: Dedicated critical listening through high-end external USB DACs.
- **Engine Behavior**:
  - Automatically acquires exclusive hardware access (**WASAPI Exclusive Event Mode** on Windows, **CoreAudio Hog Mode** on macOS, **ALSA Direct Hardware `hw:`** on Linux).
  - Programmatically reconfigures the DAC hardware clock to match the track's native sample rate (e.g. 44.1k, 48k, 88.2k, 96k, 176.4k, 192k, 352.8k, 384k).
  - **100% bypass** of all DSP, EQ, ReplayGain, and digital volume attenuation (unity gain `1.0`).
  - Bit-depth preservation: Transmits native 16-bit or 24-bit PCM words directly via DMA.

### 3.4 Mode 4: Exclusive DSP Mode (Low-Latency Hardware with EQ)
- **Target Use Case**: Audiophiles using DAC hardware clock matching who still require custom AutoEQ headphone target profiles.
- **Engine Behavior**:
  - Reconfigures hardware clock to track rate.
  - Runs Parametric EQ with automatic headroom pre-attenuation and True-Peak Limiter over exclusive hardware buffers.

---

## 4. Subsystem Architectural Specifications

### 4.1 Output-Device Model & Hot-Plug Management

1. **Continuous Device Enumeration & Hot-Plug Monitoring**:
   - Background device watcher monitors OS audio endpoints (USB DAC disconnection/connection).
   - If an active output device is disconnected during playback, the engine seamlessly pauses or migrates to the fallback system default device within 50ms without crashing or throwing audio buffer underruns.
2. **Device Capability Matrix**:
   - Probes and caches device capabilities: supported sample rates (`44100..384000`), channel counts, supported bit-depth formats (`S16LE`, `S24LE3`, `S32LE`, `F32LE`), and minimum hardware buffer sizes.

---

### 4.2 Resampling Strategy via `rubato`

When sample rate conversion is active:
- **Resampler Engine**: `rubato::SincFixedIn<f32>` (Asynchronous/Synchronous Sinc Interpolation).
- **Filter Profile**: Polyphase sinc with Kaiser window ($\beta = 9.0$, stopband attenuation > 140 dB, transition band < 5%).
- **SIMD Acceleration**: Explicit AVX2 / SSE4.1 (x86_64) and NEON (AArch64) vectorization.
- **Block-Based Streaming**: Processes audio in fixed 512-frame or 1024-frame chunks to maintain zero heap allocation during real-time playback.

---

### 4.3 DSP Ordering & Headroom Gain Staging

To eliminate clipping, phase distortion, and harmonic degradation, DSP stages must execute in strict mathematical sequence:

```
[Decoded PCM] 
    ──► [ReplayGain Normalization] (Track/Album LUFS target)
    ──► [Pre-Amp Headroom Attenuation] (-max(0, EQ_boost_dB + 1.0dB))
    ──► [Parametric Biquad EQ] (AutoEQ / User curves)
    ──► [Spatializer / Crossfeed] (Bauer binaural)
    ──► [Master Volume Fader] (Logarithmic scale)
    ──► [True-Peak Lookahead Limiter] (Ceiling: -0.2 dBFS)
    ──► [Visualizer Tap] (Lockless ring buffer feed)
    ──► [TPDF Dither] (Quantize to output word length)
    ──► [Hardware Ring Buffer]
```

#### Headroom Rule:
Prior to applying any positive EQ gain, the engine automatically attenuates the digital pre-amp by the maximum peak boost of all active filters plus 1.0 dB safety margin, guaranteeing that intermediate signals never exceed the dynamic range before the lookahead limiter.

---

### 4.4 True-Peak Lookahead Limiter Specification

- **Lookahead Delay**: 4.0 ms circular delay line (192 samples at 48 kHz).
- **Oversampling**: 4x polyphase FIR interpolation filter to detect inter-sample true peaks.
- **Envelope Follower**:
  - Attack: Instantaneous lookahead attack ($t_a = 0.1\text{ ms}$).
  - Release: Dual-stage exponential release ($t_{r1} = 50\text{ ms}$ for short transients, $t_{r2} = 250\text{ ms}$ for sustained bass passages).
- **Ceiling**: Configurable between `-0.1 dBTP` and `-1.0 dBTP` (Default: `-0.3 dBTP`).

---

### 4.5 True Gapless Playback Engine

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      GAPLESS DUAL-DECODER ARCHITECTURE                      │
│                                                                             │
│  [Decoder A: Active Track] ──► [Buffer A] ──┐                               │
│                                             ├──► [Seamless Frame Switch]    │
│  [Decoder B: Pre-buffered] ──► [Buffer B] ──┘         │                     │
│                                                       ▼                     │
│                                             [Audio Engine Ring Buffer]      │
└─────────────────────────────────────────────────────────────────────────────┘
```

1. **Encoder Priming/Delay Removal**: Decoders parse LAME/Fraunhofer/Apple metadata tags and discard encoder padding frames at start and end of tracks.
2. **Asynchronous Pre-Buffering**: When Track A has 3.0 seconds remaining, Decoder B is spawned on a worker thread to decode and prime the first 500ms of Track B into a secondary buffer.
3. **Continuous Audio Stream**: The output hardware stream is never stopped or flushed between tracks; sample $N$ of Track A is followed immediately by sample 0 of Track B in the exact same hardware audio block.

---

## 5. Measurement, Verification & Testing Methodology

To guarantee audiophile compliance, Sonora establishes four automated regression test suites:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        AUDIO VERIFICATION TEST SUITE                        │
├─────────────────────┬──────────────────┬────────────────────────────────────┤
│ Test Type           │ Metric / Tool    │ Pass Criteria                      │
├─────────────────────┼──────────────────┼────────────────────────────────────┤
│ 1. Bit-Perfect Null │ Digital Null Invert│ Residual error = $-\infty\text{ dB}$ │
│    Test             │ Bit-for-bit match│ (Exact 0 diff over 10M samples)    │
├─────────────────────┼──────────────────┼────────────────────────────────────┤
│ 2. Frequency & THD+N│ 1kHz 24-bit Sine │ THD+N < -120 dB                    │
│    Analysis         │ 20Hz-20kHz Sweep │ Passband flatness $\pm 0.05\text{ dB}$│
├─────────────────────┼──────────────────┼────────────────────────────────────┤
│ 3. Resampler SNR    │ Aliasing test    │ Stopband rejection > 140 dB        │
│    Verification     │ 44.1k -> 48k     │ Passband ripple < 0.01 dB          │
├─────────────────────┼──────────────────┼────────────────────────────────────┤
│ 4. Gapless Zero-    │ 1kHz continuous  │ Zero sample drops, no phase jump,  │
│    Discontinuity    │ split test files │ zero-crossing continuous waveform  │
└─────────────────────┴──────────────────┴────────────────────────────────────┘
```

---

## 6. Explicit Classification

### Research Evidence
- The EBU R128 and ITU-R BS.1770-4 standards specify that loudness normalization must be calculated with K-weighting filters over integrated gating windows.
- WASAPI Exclusive Event Mode is documented by Microsoft as the official method for low-latency, glitch-free audio bypass of the Windows Audio Engine.
- AES17 guidelines mandate 4x oversampling when measuring true-peak levels to prevent inter-sample clipping on DAC reconstruction filters.

### Sonora-Specific Recommendations
- Implement a dedicated native output backend (`crates/sonora-audio/src/platform/`) supporting **WASAPI Exclusive Event-Driven (Windows)**, **CoreAudio Hog Mode (macOS)**, and **ALSA Direct Hardware / PipeWire Native (Linux)**.
- Integrate `rubato` as the standard resampler throughout the entire codebase, deprecating `LinearResampler`.
- Implement automated Bit-Perfect Null regression tests in the CI pipeline (`cargo test --package sonora-audio`).

### Unresolved Questions
- Should Sonora support native DSD bitstreaming via ASIO on Windows in Phase 1?
- Should crossfeed processing default to the Chu Moy or Bauer crossfeed circuit model?
