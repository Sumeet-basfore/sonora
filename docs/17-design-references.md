# Sonora Design & Technical References

## 1. Executive Overview

This document compiles the foundational research citations, engineering standards, design system specifications, and a comprehensive comparative matrix of desktop music players. Every architectural decision in Sonora is grounded in published literature, empirical benchmarks, and international standards.

---

## 2. Comprehensive Comparative Matrix: 12 Desktop Music Players

```
┌───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                       DESKTOP MUSIC PLAYER FEATURE & ARCHITECTURE MATRIX                          │
├───────────────────────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┬───┤
│ Feature / Attribute   │ foobar│ MusicB│ Plexam│ Spotif│ AppleM│ Tauon │ Strawb│ Cider │ Amber │ Roon  │ cmus  │SON│
├───────────────────────┼───────┼───────┼───────┼───────┼───────┼───────┼───────┼───────┼───────┼───────┼───────┼───┤
│ 1. Core Language/Tech │ C++   │ C#/.NET│ReactN│Electr │Swift/C│Python │ C++/Qt│Vue/Ele│Rust/GT│ C#/.NET│ C    │Rus│
│ 2. Cross-Platform Par │ Win   │ Win   │ Cross │ Cross │ Mac/Win│Linux/W│ Cross │ Cross │ Linux │ Cross │ POSIX │Cro│
│ 3. Bit-Perfect WASAPI │ Yes   │ Yes   │ No    │ No    │ WinEx │ No    │ Yes   │ No    │ No    │ Yes   │ N/A   │YES│
│ 4. CoreAudio Hog Mode │ No    │ N/A   │ No    │ No    │ Yes   │ No    │ Yes   │ No    │ N/A   │ Yes   │ N/A   │YES│
│ 5. ALSA hw: Direct    │ N/A   │ N/A   │ No    │ N/A   │ N/A   │ No    │ Yes   │ N/A   │ No    │ Yes   │ Yes   │YES│
│ 6. Band-Limited Sinc  │ Yes   │ Yes   │ Good  │ Basic │ Basic │ Basic │ Yes   │ Basic │ Basic │ Superb│ Basic │YES│
│ 7. EBU R128 / ReplayG │ Yes   │ Yes   │ Yes   │ Basic │ SoundC│ Yes   │ Yes   │ No    │ No    │ Yes   │ Yes   │YES│
│ 8. True-Peak Limiter  │ Plugin│ Plugin│ Yes   │ Basic │ Limiter│ No   │ No    │ No    │ No    │ Yes   │ No    │YES│
│ 9. AutoEQ Profile Imp │ Manual│ Manual│ No    │ No    │ No    │ No    │ No    │ No    │ No    │ Yes   │ No    │YES│
│ 10. Sample-Rate Switch│ Yes   │ Yes   │ No    │ No    │ Yes   │ No    │ Yes   │ No    │ No    │ Yes   │ Yes   │YES│
│ 11. Syllable Karaoke  │ Plugin│ No    │ No    │ Basic │ Superb│ No    │ No    │ Superb│ No    │ No    │ No    │YES│
│ 12. Dynamic Art Mesh  │ No    │ Skin  │ Superb│ Basic │ Superb│ No    │ No    │ Superb│ Yes   │ Good  │ No    │YES│
│ 13. Milkdrop / projM  │ Plugin│ Plugin│ No    │ No    │ No    │ No    │ No    │ No    │ No    │ No    │ No    │YES│
│ 14. CAVA-Style FFT    │ Plugin│ Yes   │ Yes   │ No    │ No    │ Yes   │ Yes   │ Yes   │ No    │ No    │ Yes   │YES│
│ 15. Modular Dock Studio│Yes(CUI│Nested│ No    │ No    │ No    │ No    │ Dock  │ No    │ No    │ No    │ No    │YES│
│ 16. Plugin Sandboxing │ None  │ None  │ N/A   │ N/A   │ N/A   │ N/A   │ None  │ N/A   │ N/A   │ N/A   │ N/A   │YES│
│ 17. Extension Marketpl│ Frag  │ Forum │ None  │ Spicet│ None  │ None  │ None  │ None  │ None  │ Ext   │ None  │YES│
│ 18. Headless Daemon   │ No    │ No    │ Server│ No    │ No    │ No    │ No    │ No    │ No    │ Server│ Yes   │YES│
│ 19. Terminal TUI Cli  │ No    │ No    │ No    │ No    │ No    │ No    │ No    │ No    │ No    │ No    │ Pure  │YES│
│ 20. RAM Footprint (MB)│ 20-50 │ 60-120│150-300│500-150│300-800│50-100 │ 80-150│400-900│ 40-70 │400-120│ 10-25 │30-│
│ 21. Sub-ms Large Search│Yes   │ Yes   │ No    │ Cloud │ Med   │ Fast  │ Fast  │ Slow  │ N/A   │ Fast  │ Fast  │YES│
│ 22. Multilingual Lyric│ Plugin│ No    │ No    │ No    │ Yes   │ No    │ No    │ Yes   │ No    │ No    │ No    │YES│
│ 23. Lyrics Offset Edit│ Plugin│ Plugin│ No    │ No    │ No    │ Yes   │ No    │ No    │ No    │ No    │ No    │YES│
│ 24. Gapless Playback  │ Flaw  │ Flaw  │ Sweet │ Basic │ Good  │ Good  │ Good  │ Basic │ Basic │ Flaw  │ Good  │YES│
│ 25. Open-Source       │ No    │ No    │ No    │ No    │ No    │ Yes   │ Yes   │ Dual  │ Yes   │ No    │ Yes   │YES│
└───────────────────────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴───┘
```
*(Legend: SON = Sonora Target; foobar = foobar2000; MusicB = MusicBee; Plexam = Plexamp; Spotif = Spotify; AppleM = Apple Music; Tauon = Tauon Music Box; Strawb = Strawberry; Amber = Amberol; Rus = Rust; Cro = Cross-Platform; YES = Native First-Class Support)*

---

## 3. Audio & DSP Standards Bibliography

### 3.1 Loudness & Normalization Standards
1. **International Telecommunication Union (ITU)**:
   - *Recommendation ITU-R BS.1770-4*: "Algorithms to measure audio programme loudness and true-peak audio level." Defines K-weighting pre-filter, RLB weighting curves, and gated integrated loudness measurement algorithms.
   - *Recommendation ITU-R BS.1387-1*: "Method for objective measurements of perceived audio quality (PEAQ)."
2. **European Broadcasting Union (EBU)**:
   - *EBU Tech 3341 & 3342*: "Loudness Metering: 'EBU Mode' practical metering specs & Loudness Range (LRA)." Defines the $-18.0\text{ LUFS}$ target for audio delivery and $\pm 0.5\text{ LU}$ tolerance windows.
3. **ReplayGain 2.0 Specification**:
   - Standardized loudness calculation based on 100ms RMS blocks and equal-loudness contour filtering, with metadata tags (`REPLAYGAIN_TRACK_GAIN`, `REPLAYGAIN_ALBUM_GAIN`, `REPLAYGAIN_TRACK_PEAK`).

---

### 3.2 Digital Audio Resampling & Filter Theory
1. **Smith, Julius O. (Stanford CCRMA)**:
   - *"Digital Audio Resampling Home Page"* & *"Bandlimited Interpolation - Introduction and Algorithm"*. Mathematical basis for polyphase windowed-sinc interpolation, Kaiser window design ($\beta=9.0$), and alias-free fractional rate conversion.
2. **Bristow-Johnson, Robert**:
   - *"Cookbook formulae for audio equalizer biquad filter coefficients"*. Standardized derivation of peaking EQ, low-shelf, high-shelf, bandpass, notch, and allpass biquad transfer functions in Direct Form II Transposed architectures.
3. **Audio Engineering Society (AES)**:
   - *AES17 Standard*: "Measurement of digital audio equipment." Formally specifies $4\times$ oversampling for accurate inter-sample true peak detection.

---

### 3.3 Platform Native Audio APIs
1. **Microsoft Windows WASAPI**:
   - *Core Audio APIs Documentation (Microsoft Learn)*: Exclusive-Mode Streams with `AUDCLNT_STREAMFLAGS_EVENTCALLBACK`, `IAudioClient::Initialize`, and `IAudioRenderClient`.
2. **Apple macOS CoreAudio HAL**:
   - *CoreAudio Hardware Abstraction Layer (HAL) Reference*: `AudioHardwareService`, `kAudioDevicePropertyHogMode`, and `kAudioDevicePropertyNominalSampleRate`.
3. **Linux PipeWire & ALSA**:
   - *PipeWire SPA (Simple Plugin API) & ALSA Direct PCM*: `snd_pcm_open` with direct hardware device nodes (`hw:CARD,DEV`) bypassing `dmix` software mixing.

---

## 4. UI/UX & Theming Literature

### 4.1 Dynamic Palette Extraction & Color Spaces
1. **Google Material Design 3 (Material You / Monet Engine)**:
   - *"Color and Theming in M3"*: Algorithmic extraction of tonal palettes using HCT (Hue, Chroma, Tone) color space, ensuring strict WCAG 2.1 AA/AAA contrast ratios between extracted accent tones and background surfaces.
2. **W3C Web Content Accessibility Guidelines (WCAG 2.1)**:
   - *Success Criterion 1.4.3 & 1.4.6*: Contrast (Minimum 4.5:1 for normal text, 3:1 for large text and UI components).

### 4.2 Synchronized Lyrics & Kinetic Typography
1. **W3C Timed Text Markup Language (TTML2) & IMSC1**:
   - Synchronized text representation standard for word-level and syllable-level timestamps (`begin`, `end`, `dur`), furigana rubies, and karaoke fill animations.
2. **LRCLIB Community Project**:
   - *Open Synced Lyrics API Specification*: RESTful standard for querying and contributing community-verified synced LRC and syllable metadata.

---

## 5. Visualizer & DSP References

1. **projectM / Milkdrop 2**:
   - *Milkdrop 2 Preset Authoring Guide & libprojectM*: Algorithmic procedural shader equations evaluating per-frame and per-vertex mathematical expressions over audio waveform/FFT buffers.
2. **CAVA (Console-based Audio Visualizer for ALSA)**:
   - Logarithmic audio frequency distribution algorithms, integral gravity falloff curves, and stereo phase correlation.

---

## 6. Explicit Classification

### Research Evidence
- Academic and industry consensus confirms that band-limited sinc interpolation (`rubato` / SoX) is the only mathematically transparent method for digital sample rate conversion, providing $>140\text{ dB}$ signal-to-noise ratio.
- The W3C TTML2 and LRCLIB standards represent the most robust, open formats for multi-lingual and word-level synchronized lyrics.

### Sonora-Specific Recommendations
- Adopt the HCT / WCAG 2.1 contrast model for Sonora's dynamic album art color extraction engine.
- Benchmark Sonora's DSP biquad filters directly against the Bristow-Johnson Audio EQ Cookbook formulas.

### Unresolved Questions
- Should Sonora implement custom WebGPU compute shaders for projectM preset evaluation or embed `libprojectM` C++ bindings?
- How should multi-channel 5.1/7.1 audio be fold-down downmixed to stereo for headphone binaural listening (ITU-R BS.775 vs custom HRTF)?
