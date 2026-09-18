use sonora_common::{Result, SonoraError};
use std::fs::File;
use std::path::{Path, PathBuf};
use symphonia::core::audio::Channels;
use symphonia::core::codecs::audio::{AudioDecoder as SymphoniaAudioDecoder, AudioDecoderOptions};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;

/// Audio stream properties discovered after probing an audio file.
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub sample_rate: u32,
    pub channels: u32,
    pub duration_frames: Option<u64>,
}

/// Symphonia-backed audio decoder for file playback.
pub struct AudioDecoder {
    path: PathBuf,
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn SymphoniaAudioDecoder>,
    track_id: u32,
    raw_sample_buf: Vec<f32>,
    stereo_sample_buf: Vec<f32>,
    info: StreamInfo,
    eof_reached: bool,
}

struct OpenedAudioStream {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn SymphoniaAudioDecoder>,
    track_id: u32,
    info: StreamInfo,
}

impl AudioDecoder {
    /// Open and probe an audio file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let opened = Self::open_reader_and_decoder(&path_buf)?;

        Ok(Self {
            path: path_buf,
            format_reader: opened.format_reader,
            decoder: opened.decoder,
            track_id: opened.track_id,
            raw_sample_buf: Vec::new(),
            stereo_sample_buf: Vec::new(),
            info: opened.info,
            eof_reached: false,
        })
    }

    fn open_reader_and_decoder(path: &Path) -> Result<OpenedAudioStream> {
        let file = File::open(path)
            .map_err(|e| SonoraError::Audio(format!("Failed to open file: {e}")))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            hint.with_extension(ext);
        }

        let format_reader = symphonia::default::get_probe()
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| SonoraError::Audio(format!("Probe error: {e}")))?;

        let track = format_reader
            .default_track(TrackType::Audio)
            .ok_or_else(|| SonoraError::Audio("No supported audio track found".to_string()))?
            .clone();

        let track_id = track.id;
        let audio_params = match &track.codec_params {
            Some(CodecParameters::Audio(params)) => params,
            _ => {
                return Err(SonoraError::Audio(
                    "No audio codec parameters found".to_string(),
                ))
            }
        };

        let sample_rate = audio_params.sample_rate.unwrap_or(44100);
        let channels = audio_params
            .channels
            .as_ref()
            .map(|c: &Channels| c.count() as u32)
            .unwrap_or(2);
        let duration_frames = track.num_frames;

        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(audio_params, &AudioDecoderOptions::default())
            .map_err(|e| SonoraError::Audio(format!("Failed to initialize decoder: {e}")))?;

        let info = StreamInfo {
            sample_rate,
            channels,
            duration_frames,
        };

        Ok(OpenedAudioStream {
            format_reader,
            decoder,
            track_id,
            info,
        })
    }

    pub fn info(&self) -> &StreamInfo {
        &self.info
    }

    /// Seek to a specific timestamp in milliseconds.
    pub fn seek(&mut self, position_ms: u64) -> Result<()> {
        let seconds = (position_ms / 1000) as i64;
        let nanos = ((position_ms % 1000) * 1_000_000) as u32;
        let time = Time::try_new(seconds, nanos).unwrap_or(Time::ZERO);

        let seek_res = if self.eof_reached {
            // Re-open fresh stream if EOF was reached to avoid container state exhaustion
            match Self::open_reader_and_decoder(&self.path) {
                Ok(opened) => {
                    self.format_reader = opened.format_reader;
                    self.decoder = opened.decoder;
                    self.track_id = opened.track_id;
                    self.info = opened.info;
                    self.format_reader.seek(
                        SeekMode::Accurate,
                        SeekTo::Time {
                            time,
                            track_id: Some(self.track_id),
                        },
                    )
                }
                Err(e) => Err(SymphoniaError::IoError(std::io::Error::other(
                    e.to_string(),
                ))),
            }
        } else {
            let res = self.format_reader.seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            );
            if res.is_err() {
                // Fallback: reopen from path and seek
                if let Ok(opened) = Self::open_reader_and_decoder(&self.path) {
                    self.format_reader = opened.format_reader;
                    self.decoder = opened.decoder;
                    self.track_id = opened.track_id;
                    self.info = opened.info;
                    self.format_reader.seek(
                        SeekMode::Accurate,
                        SeekTo::Time {
                            time,
                            track_id: Some(self.track_id),
                        },
                    )
                } else {
                    res
                }
            } else {
                res
            }
        };

        seek_res.map_err(|e| SonoraError::Audio(format!("Seek error: {e}")))?;

        self.eof_reached = false;
        self.decoder.reset();
        self.raw_sample_buf.clear();
        self.stereo_sample_buf.clear();
        Ok(())
    }

    /// Decode the next packet of audio and return a slice of interleaved stereo (2-channel) f32 samples.
    pub fn decode_next_stereo(&mut self) -> Result<Option<&[f32]>> {
        loop {
            let packet = match self.format_reader.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    self.eof_reached = true;
                    return Ok(None);
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(SymphoniaError::DecodeError(msg)) => {
                    tracing::warn!("Recoverable demuxer error: {msg}");
                    continue;
                }
                Err(e) => return Err(SonoraError::Audio(format!("Format reader error: {e}"))),
            };

            if packet.track_id != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(audio_buf) => {
                    let total_samples = audio_buf.samples_interleaved();
                    self.raw_sample_buf.resize(total_samples, 0.0);
                    audio_buf.copy_to_slice_interleaved(&mut self.raw_sample_buf);

                    let channels = self.info.channels.max(1) as usize;
                    let frames = total_samples / channels;

                    if channels == 1 {
                        // Mono -> Stereo duplicate
                        self.stereo_sample_buf.resize(frames * 2, 0.0);
                        for i in 0..frames {
                            let s = self.raw_sample_buf[i];
                            self.stereo_sample_buf[i * 2] = s;
                            self.stereo_sample_buf[i * 2 + 1] = s;
                        }
                    } else if channels == 2 {
                        // Already stereo
                        self.stereo_sample_buf.resize(total_samples, 0.0);
                        self.stereo_sample_buf.copy_from_slice(&self.raw_sample_buf);
                    } else {
                        // Multi-channel downmix to stereo
                        self.stereo_sample_buf.resize(frames * 2, 0.0);
                        for i in 0..frames {
                            let l = self.raw_sample_buf[i * channels];
                            let r = self.raw_sample_buf[i * channels + 1];
                            self.stereo_sample_buf[i * 2] = l;
                            self.stereo_sample_buf[i * 2 + 1] = r;
                        }
                    }

                    return Ok(Some(&self.stereo_sample_buf));
                }
                Err(SymphoniaError::DecodeError(msg)) => {
                    tracing::warn!("Recoverable decode error: {msg}");
                    continue;
                }
                Err(e) => return Err(SonoraError::Audio(format!("Decoder fatal error: {e}"))),
            }
        }
    }
}
