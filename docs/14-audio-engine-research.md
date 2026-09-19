# Sonora Audio Engine Research: Audiophile Pipelines, Platform APIs & Rust Ecosystem

## 1. Executive Overview

This document presents a comprehensive technical investigation into high-fidelity audio reproduction, low-latency DSP pipelines, platform-native output backends, and the modern Rust audio ecosystem. It concludes with a thorough audit of Sonora's current audio crate architecture (`crates/sonora-audio` and `crates/sonora-dsp`), identifying concrete technical deficiencies and defining the architectural requirements for bit-perfect and audiophile-grade playback.

---

## 2. Deep-Dive: Core Audio Pipeline Concepts

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          IDEAL AUDIOPHILE SIGNAL PATH                       │
│                                                                             │
│  [Source / File]                                                            │
│         │                                                                   │
│         ▼                                                                   │
│  [Decoder (Symphonia)] ──► Interleaved PCM / Float32 Samples                │
│         │                                                                   │
│         ▼                                                                   │
│  [Channel Matrix / Upmix / Downmix]                                         │
│         │                                                                   │
│         ▼                                                                   │
│  [Sample Rate Handler] ──► (Bit-Perfect Passthrough OR Sinc Resampler)      │
│         │                                                                   │
│         ▼                                                                   │
│  [ReplayGain / EBU R128 Stage] ──► (Track/Album Gain + Pre-amp Attenuation) │
│         │                                                                   │
│         ▼                                                                   │
│  [Parametric EQ & DSP Graph] ──► (Biquad Cascades / AutoEQ / Crossfeed)     │
│         │                                                                   │
│         ▼                                                                   │
│  [True Peak Lookahead Limiter] ──► (Prevents Inter-Sample Clipping)         │
│         │                                                                   │
│         ▼                                                                   │
│  [Dither Stage (if truncating to 16/24-bit int)]                            │
│         │                                                                   │
│         ▼                                                                   │
│  [Audio Output Backend] ──► (WASAPI Exclusive / ALSA hw / CoreAudio Hog)    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### 2.1 Bit-Perfect Playback: Theory and Requirements

Bit-perfect playback occurs when the exact PCM samples output by the decoder are transmitted to the Digital-to-Analog Converter (DAC) without any mathematical modification, volume attenuation, mixing, or rate conversion.

#### Key Conditions for Bit-Perfect Reproduction:
1. **OS Mixer Bypass**: Bypassing Windows Audio Engine (APO/Audio Processing Objects), macOS CoreAudio mixer, and PulseAudio/PipeWire default resampling graphs.
2. **Hardware Clock Reconfiguration**: Reconfiguring the DAC's hardware clock to match the native sample rate of the stream (e.g., switching between 44.1 kHz, 48.0 kHz, 96.0 kHz, 192.0 kHz, 384.0 kHz, and DSD).
3. **Integer Preservation & Dithering**: If the output device requires 16-bit or 24-bit integer words, floating-point samples must undergo TPDF (Triangular Probability Density Function) dither rather than naive truncation.
4. **DSP & Volume Bypass**: In strict bit-perfect mode, digital volume controls, equalizers, and replaygain stages must be 100% bypassed (unity gain `1.0`), delegating volume attenuation to the physical DAC or amplifier.

---

### 2.2 Platform Output Backends: Shared vs Exclusive Mode

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       PLATFORM AUDIO BACKEND ARCHITECTURE                   │
├───────────────┬────────────────────────────┬────────────────────────────────┤
│ Platform      │ Shared Mode (Default)      │ Exclusive / Bit-Perfect Mode   │
├───────────────┼────────────────────────────┼────────────────────────────────┤
│ **Windows**   │ WASAPI Shared Mode         │ WASAPI Exclusive Event-Driven  │
│               │ • Mixed with system sounds │ • Direct hardware latching     │
│               │ • Fixed OS sample rate     │ • Auto sample-rate switching   │
│               │ • High latency (20-50ms)   │ • Ultra-low latency (2-5ms)    │
│               │ • Windows APO processing   │ • ASIO Driver (Alternative)    │
├───────────────┼────────────────────────────┼────────────────────────────────┤
│ **macOS**     │ CoreAudio Default Stream   │ CoreAudio Hog Mode             │
│               │ • System mixer active      │ • `kAudioDevicePropertyHogMode`│
│               │ • Fixed device sample rate │ • Direct nominal rate control  │
│               │ • Clean float pipeline     │ • Zero system sound mixing     │
├───────────────┼────────────────────────────┼────────────────────────────────┤
│ **Linux**     │ PipeWire / PulseAudio      │ ALSA Direct Hardware (`hw:X,Y`)|
│               │ • PipeWire dynamic graph   │ • Direct DMA to soundcard      │
│               │ • Configurable rates       │ • Bypasses `dmix` and servers  │
│               │ • Low latency JACK bridge  │ • Full rate/format ownership   │
└───────────────┴────────────────────────────┴────────────────────────────────┘
```

#### Detailed Platform Nuances:
- **Windows WASAPI Exclusive**:
  - *Event-Driven Mode*: The hardware driver triggers a kernel event when a new buffer is required. Far superior to timer-driven push mode in eliminating jitter and buffer underruns.
  - *Sample-Rate Switching*: Changing sample rates requires tearing down the `IAudioClient` stream, re-initializing with `AUDCLNT_STREAMFLAGS_EVENTCALLBACK`, and re-registering buffers.
- **macOS CoreAudio**:
  - macOS possesses a high-quality 32-bit float mixer by default, but to prevent OS sample-rate conversion, an audiophile player must programmatically set `kAudioDevicePropertyNominalSampleRate` on the `AudioDeviceID` and acquire `kAudioDevicePropertyHogMode`.
- **Linux PipeWire vs ALSA `hw:`**:
  - While raw ALSA `hw:CARD,DEV` provides absolute exclusivity, modern PipeWire (0.3+) supports dynamic rate switching when applications specify allowed sample rates in their stream properties (`node.rate`, `playback.props`), allowing clean audiophile playback without completely locking the desktop sound server.

---

### 2.3 Sample-Rate Switching & Hardware Clock Latching

When transitioning between tracks of differing sample rates (e.g., a 44.1 kHz FLAC followed by a 96.0 kHz 24-bit Hi-Res track):
- **DAC Relay Latching**: Physical DACs switch hardware clock crystals (e.g., 22.5792 MHz for 44.1k multiples vs 24.576 MHz for 48k multiples). This mechanical or electrical transition takes 50ms–300ms.
- **Popping / Artifact Mitigation**: The audio engine must inject a brief 50ms–100ms mute/zero-pad buffer during the rate renegotiation to prevent loud acoustic pops and relay clicks from reaching the speakers.

---

### 2.4 Resampling Quality: Sinc vs Linear Interpolation

When sample rate conversion is necessary (e.g., in Shared Mode where the hardware is locked to 48.0 kHz but the track is 44.1 kHz):

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           RESAMPLER COMPARISON                              │
├─────────────────────┬─────────────────┬──────────────┬──────────────────────┤
│ Method              │ SNR / THD+N     │ Aliasing     │ Computational Cost   │
├─────────────────────┼─────────────────┼──────────────┼──────────────────────┤
│ Linear Resampling   │ Poor (< 60 dB)  │ Severe       │ Extremely Low        │
│ Cubic Hermite       │ Medium (~80 dB) │ Moderate     │ Low                  │
│ Band-Limited Sinc   │ Superb (>140 dB)│ Zero (< -140dB) High (SIMD optimized)│
│ (Rubato / SoX)      │                 │              │                      │
└─────────────────────┴─────────────────┴──────────────┴──────────────────────┘
```

- **Linear Resampling**: Introduces intense high-frequency aliasing distortion and high-end frequency rolloff. Completely unacceptable for audiophile listening.
- **Band-Limited Sinc Resampling (`rubato`)**: Utilizes polyphase filterbanks with Kaiser or Blackman-Harris windowed sinc functions. Preserves total passband flatness up to 20 kHz with stopband rejection exceeding -140 dB.

---

### 2.5 Loudness Normalization: ReplayGain 2.0 & EBU R128

- **EBU R128 / ITU-R BS.1770-4 Standard**: Computes integrated loudness using K-weighting filters measured in LUFS (Loudness Units relative to Full Scale). Target reference level: **-18.0 LUFS** (or classic ReplayGain **-14.0 LUFS / 89 dB SPL**).
- **Track vs Album Gain Modes**:
  - *Track Gain*: Normalizes every track to the exact target level (ideal for random shuffle playlists).
  - *Album Gain*: Applies a uniform gain offset across an entire album to preserve intentional dynamic differences between loud anthems and quiet acoustic interludes.
- **Peak Headroom & Pre-Amp**:
  - Applying positive ReplayGain on dynamic tracks can cause digital clipping.
  - The engine must read the `replaygain_track_peak` / `replaygain_album_peak` metadata and calculate `effective_gain = min(target_gain, 1.0 / peak)` when clipping prevention is enabled.

---

### 2.6 Equalizer Ordering, Biquad DSP & Phase Behavior

- **Parametric Biquad Filters**: Second-order IIR filters (Direct Form II Transposed).
- **Denormal Float Prevention**: In low-signal or silent passages, IIR filter feedback buffers decay into floating-point subnormal numbers, causing massive CPU spikes (10x–100x slowdown). The engine must add micro-denormal offsets (`1e-25f`) or flush denormals to zero via FTZ/DAZ CPU flags (`_MM_SET_FLUSH_ZERO_MODE`).
- **Phase & Headroom Management**:
  - Boosting any frequency band increases peak amplitude.
  - An automatic pre-cut attenuation (`-max_boost dB`) must be applied prior to the EQ stage to guarantee floating-point headroom before reaching the output limiter.

---

### 2.7 Limiter Design: True Peak Lookahead vs Hard Clipping

- **Hard Clipping**: Clamping samples with `sample.clamp(-1.0, 1.0)` generates severe square-wave odd-harmonic distortion and harsh audio artifacts.
- **True Peak Lookahead Limiter**:
  - *Lookahead Buffer*: 2ms–5ms circular delay buffer.
  - *4x Oversampling Detector*: Upsamples audio to detect inter-sample peaks that cross 0.0 dBFS after DAC reconstruction.
  - *Smooth Gain Envelope*: Fast logarithmic attack (0.1ms) with smooth exponential/program-dependent release curve (50ms–200ms) to ensure transparent, artifact-free volume leveling without pumping.

---

### 2.8 True Gapless Playback

Gapless playback requires seamless, sample-accurate transition between consecutive audio files.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          GAPLESS PLAYBACK LIFECYCLE                         │
│                                                                             │
│  Track A [Header Padding] [======= Real Audio Payload =======] [End Padding]│
│                                              │                              │
│                                  Cross-fused at zero-sample                 │
│                                              ▼                              │
│  Track B              [Header Padding] [======= Real Audio Payload =======] │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Requirements:
1. **Encoder Delay & Padding Trimming**: Lossy formats (MP3, AAC) introduce encoder priming delays (typically 576–1152 samples) and bit-reservoir padding at the end. The decoder must read LAME/iTunSMPB tags and strip these silent samples.
2. **Dual-Decoder Ring Buffer**: As Track A approaches its final 2 seconds, the audio engine asynchronously initializes the decoder for Track B, pre-filling a secondary ring buffer.
3. **Continuous Clock**: The real-time output stream callback is never stopped or flushed between tracks. The audio thread seamlessly pulls the first sample of Track B in the exact frame following Track A's last valid sample.

---

## 3. Evaluation of the Rust Audio Ecosystem

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          RUST AUDIO CRATE MATRIX                            │
├───────────────┬────────────────────────────┬────────────────────────────────┤
│ Crate         │ Capabilities               │ Limitations & Gaps             │
├───────────────┼────────────────────────────┼────────────────────────────────┤
│ **CPAL**      │ Cross-platform abstraction │ No WASAPI Exclusive Event Mode │
│               │ (WASAPI, CoreAudio, ALSA)  │ No macOS Hog Mode control      │
│               │ Easy stream setup          │ Inflexible device rate hooks   │
├───────────────┼────────────────────────────┼────────────────────────────────┤
│ **Symphonia** │ 100% safe pure-Rust audio  │ Lacks native DSD / SACD format │
│               │ decoding (FLAC, MP3, AAC,  │ Gapless metadata parsing needs │
│               │ ALAC, Vorbis, Opus, WAV)   │ custom sample trim wrapper     │
├───────────────┼────────────────────────────┼────────────────────────────────┤
│ **rubato**    │ High-performance Sinc and  │ Requires careful block-size    │
│               │ FFT resamplers with SIMD   │ management in real-time streams│
├───────────────┼────────────────────────────┼────────────────────────────────┤
│ **rtrb**      │ Real-time lock-free single-│ Fixed-capacity ring buffer     │
│               │ producer single-consumer   │ (ideal for audio thread feed)  │
├───────────────┼────────────────────────────┼────────────────────────────────┤
│ **biquad**    │ Standard biquad filter     │ Needs denormal protection      │
│               │ coefficients and formulas  │ wrapper for audio engine       │
└───────────────┴────────────────────────────┴────────────────────────────────┘
```

---

## 4. Audit of Sonora's Current Audio Architecture

We audited the current implementation in `crates/sonora-audio` and `crates/sonora-dsp`.

### Concrete Technical Deficiencies Identified:

1. **Severe Resampling Distortion (`LinearResampler`)**:
   - `crates/sonora-dsp/src/resampler.rs` uses basic linear interpolation. Linear resampling produces significant high-frequency aliasing distortion and inter-modulation artifacts, degrading sound quality when converting 44.1 kHz to 48.0 kHz.
2. **Harmonic Distortion via Hard-Clipping Limiter (`HardLimiter`)**:
   - `crates/sonora-dsp/src/limiter.rs` simply clamps samples using `sample.clamp(-self.ceiling, self.ceiling)`. Any signal peak exceeding 0 dBFS generates harsh square-wave harmonic distortion instead of transparent lookahead gain reduction.
3. **No Bit-Perfect / Exclusive Hardware Output**:
   - `crates/sonora-audio/src/output.rs` relies entirely on CPAL's default device stream, running through the operating system's shared mixer. There is no support for WASAPI Exclusive Event Mode, macOS Hog Mode, or ALSA Direct Hardware (`hw:`).
4. **Broken Gapless Playback**:
   - In `crates/sonora-audio/src/player.rs`, when a track finishes or changes, `engine.flush()` is called, wiping all buffered audio and pausing the stream before opening the next track. This produces audible gaps, clicks, and latency between consecutive album tracks.
5. **Missing ReplayGain / EBU R128 Processing**:
   - Neither `sonora-audio` nor `sonora-dsp` parses or applies ReplayGain track/album gain or peak tags from the decoded audio files.
6. **Lack of Inter-Sample Peak Headroom in DSP Chain**:
   - The equalizer operates without dynamic pre-cut attenuation or true-peak monitoring, creating potential digital clipping when multiple EQ bands are boosted.
7. **Single-Thread Decoder Stalling**:
   - The worker loop in `player.rs` performs file opening, tag parsing, decoding, and resampling sequentially on a single thread. Slow disk I/O or network shares can starve the real-time ring buffer.

---

## 5. Explicit Classification

### Research Evidence
- Benchmark tests across audio research literature demonstrate that linear interpolation achieves less than 60 dB Signal-to-Noise Ratio (SNR), whereas polyphase Sinc resampling (`rubato`) achieves >140 dB SNR with total passband fidelity.
- True peak inter-sample overshoots routinely reach +0.5 dB to +3.0 dBFS on modern mastered recordings when converted back to analog. Lookahead limiting with 4x oversampling is necessary to prevent DAC reconstruction clipping.

### Sonora-Specific Recommendations
- Replace `LinearResampler` with `rubato` Sinc resampler across all shared-mode playback pipelines.
- Implement an explicit **Dual-Decoder Pre-buffering Queue** in `AudioPlayer` to deliver true sample-accurate gapless transitions.
- Build a native backend abstraction tier supporting **WASAPI Exclusive Event Mode (Windows)**, **CoreAudio Hog Mode (macOS)**, and **PipeWire/ALSA Direct (Linux)** alongside the standard CPAL shared backend.
- Replace `HardLimiter` with an oversampled true-peak lookahead limiter with configurable release physics.

### Unresolved Questions
- Should Sonora support native DSD (Direct Stream Digital) bitstreaming / DoP (DSD over PCM) in Phase 1 or Phase 2?
- For Linux exclusive mode, should PipeWire dynamic rate matching take precedence over raw ALSA `hw:` locking?
