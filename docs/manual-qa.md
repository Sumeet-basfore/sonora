# Sonora Pre-Release Manual QA Checklist & Verification Log

This checklist documents the manual QA test procedures and observations validating Sonora v0.1.0 across desktop and terminal interfaces.

---

## 1. First Launch & Empty Library Experience
- [x] **Clean Startup:** Launch Sonora on a clean environment without prior database configuration.
- [x] **Empty State UI:** Verified the Library view renders an informative, centered empty state with clear illustration, explanation, and an actionable "Scan Music Directory" primary button.
- [x] **Directory Picker:** Verified clicking "Scan Music Directory" prompts the directory picker or opens the Scan modal with the directory field ready.

---

## 2. Audio Library Scanning & Metadata Indexing
- [x] **Format Ingestion:** Scanned `/tmp/sonora_test_audio` containing FLAC (16/24-bit), MP3, WAV (stereo & mono), OGG Vorbis, AAC/M4A, and ALAC.
- [x] **Progress Indicators:** Verified scanning logs and displays indexed tracks count without blocking the main UI thread.
- [x] **FTS5 Search:** Verified sub-millisecond search across track titles, artist names, and album titles with partial strings, special punctuation (`AC/DC`, `AT&T`), and quoted terms.
- [x] **Library Pruning:** Verified deleting a file and rescanning prunes deleted track entries from SQLite FTS5 index.
- [x] **Symlink Safety:** Verified cyclical directory symlinks are detected and skipped without recursion loops.

---

## 3. Real Audio Playback & Output Verification
- [x] **FLAC (16-bit / 44.1kHz):** Decodes 264,600 samples; output is clear and pitch-accurate.
- [x] **FLAC (24-bit / 96kHz High-Res):** Decodes 384,000 samples; continuous linear resampling performs rate conversion to device rate without clicks or artifacts.
- [x] **MP3 (CBR/VBR):** Decodes 264,600 samples; smooth playback from start to finish.
- [x] **OGG (Vorbis):** Decodes 264,600 samples; stereo separation intact.
- [x] **AAC / M4A (ISOMP4):** Decodes 268,288 samples; packet timing and header synchronization verified.
- [x] **ALAC (Apple Lossless in M4A):** Decodes 264,600 samples; lossless fidelity verified.
- [x] **WAV (PCM Stereo & Mono):** Mono PCM auto-duplicates to dual-channel stereo without single-ear audio; 16/24/32-bit plays without clipping.
- [x] **Audible Output:** Real-time CPAL audio thread renders samples into hardware device with zero mutex contention.

---

## 4. Playback Controls & Transport
- [x] **Play / Pause / Resume:** Spacebar and UI play/pause button toggle state instantly without audio popping.
- [x] **Rapid Play/Pause:** Rapid toggling (10+ times in 3 seconds) maintains atomic state coherence without deadlock or thread starvation.
- [x] **Seeking (Mid-Track):** Dragging seek slider repositions playback instantly; sample decoding resumes accurately.
- [x] **Seeking (Near End & Post-EOF):** Seeks at EOF on ISOMP4/M4A streams automatically re-open and reposition decoder without stream exhaustion errors.
- [x] **Volume & Mute:** Linear volume slider and mute toggle provide gain adjustment with smooth ramping.
- [x] **Next / Previous:** Next and Previous buttons cycle queue cleanly and reset decoder state.

---

## 5. Queue Management & Transitions
- [x] **Sequential Playback:** Track completion triggers automatic transition to next queued track.
- [x] **Queue Drawer:** Queue drawer (`q` / icon) displays track index, active track indicator, and track duration.
- [x] **Reordering & Removal:** Items can be moved up, moved down, and removed without interrupting current playing audio.
- [x] **End of Queue:** Cleanly transitions to Stopped when all items finish.

---

## 6. Real-Time Visualizer & DSP
- [x] **Visualizer Toggle:** `v` key / icon toggles FFT spectrum visualizer.
- [x] **Spectrum Activity:** Frequency bins react dynamically to audio content off the real-time thread.
- [x] **Zero-Allocation Safety:** Visualizer tap uses atomic ring buffer with zero allocations on the audio callback.

---

## 7. Synchronized Lyrics & Artwork
- [x] **LRC Synchronization:** Lyrics view (`l` / icon) highlights active line and scrolls with playback timestamp.
- [x] **Sidecar Fallback:** Automatically resolves `.lrc` files in the same directory as audio tracks.
- [x] **Artwork Cache:** Embedded ID3/FLAC covers and `cover.jpg` load into the disk cache and render in playback bar, album detail, and grid cards.

---

## 8. Extensions & Marketplace UX
- [x] **Browse Catalog:** Extensions view displays plugins and themes with filter/search support.
- [x] **Installation Flow:** Permission overview dialog presents requested capabilities (`lyrics:provider`, `visualizer:tap`, `network:fetch`) before confirmation.
- [x] **Offline Handling:** Unreachable registry gracefully presents cached extensions and informative notices without raw error strings.
- [x] **Updates & Rollback:** Plugin update and rollback restore previous version receipts cleanly.

---

## 9. Error Recovery & Edge Cases
- [x] **Missing Audio File:** Missing files report clean error and do not crash the engine.
- [x] **Corrupt Audio File:** Zero-byte or invalid headers are safely skipped and logged.
- [x] **Output Device Disconnection:** Audio engine handles CPAL buffer stream errors without application panic.

---

## 10. Application Restart & Persistence
- [x] **State Persistence:** Custom themes, active layout presets, and volume settings persist across app restarts in local storage and config files.
- [x] **Library Preservation:** SQLite FTS5 database persists across sessions without requiring full rescans on every launch.
