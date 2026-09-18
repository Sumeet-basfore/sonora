use sonora_common::{Result, SonoraError};
use std::fs::File;
use std::path::Path;
use symphonia::core::codecs::audio::{AudioDecoder as SymphoniaAudioDecoder, AudioDecoderOptions};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Audio stream properties discovered after probing an audio file.
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub sample_rate: u32,
    pub channels: u32,
    pub duration_frames: Option<u64>,
}

/// Symphonia-backed audio decoder for file playback.
pub struct AudioDecoder {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn SymphoniaAudioDecoder>,
    track_id: u32,
    sample_buf: Vec<f32>,
    info: StreamInfo,
}

impl AudioDecoder {
    /// Open and probe an audio file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .map_err(|e| SonoraError::Audio(format!("Failed to open file: {e}")))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.as_ref().extension().and_then(|s| s.to_str()) {
            hint.with_extension(ext);
        }

        let format_reader = symphonia::default::get_probe()
            .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
            .map_err(|e| SonoraError::Audio(format!("Probe error: {e}")))?;

        let track = format_reader
            .default_track(TrackType::Audio)
            .ok_or_else(|| SonoraError::Audio("No supported audio track found".to_string()))?
            .clone();

        let track_id = track.id;
        let audio_params = match &track.codec_params {
            Some(CodecParameters::Audio(params)) => params,
            _ => return Err(SonoraError::Audio("No audio codec parameters found".to_string())),
        };

        let sample_rate = audio_params.sample_rate.unwrap_or(44100);
        let channels = audio_params
            .channels
            .as_ref()
            .map(|c| c.count() as u32)
            .unwrap_or(2);
        let duration_frames = audio_params.max_frames_per_packet;

        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(audio_params, &AudioDecoderOptions::default())
            .map_err(|e| SonoraError::Audio(format!("Failed to initialize decoder: {e}")))?;

        Ok(Self {
            format_reader,
            decoder,
            track_id,
            sample_buf: Vec::new(),
            info: StreamInfo {
                sample_rate,
                channels,
                duration_frames,
            },
        })
    }

    pub fn info(&self) -> &StreamInfo {
        &self.info
    }

    /// Decode the next packet of audio, filling the internal sample buffer and returning a slice of interleaved f32 PCM samples.
    pub fn decode_next(&mut self) -> Result<Option<&[f32]>> {
        loop {
            let packet = match self.format_reader.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => return Ok(None),
                Err(SymphoniaError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(e) => return Err(SonoraError::Audio(format!("Format reader error: {e}"))),
            };

            if packet.track_id != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(audio_buf) => {
                    self.sample_buf.resize(audio_buf.samples_interleaved(), 0.0);
                    audio_buf.copy_to_slice_interleaved(&mut self.sample_buf);
                    return Ok(Some(&self.sample_buf));
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
