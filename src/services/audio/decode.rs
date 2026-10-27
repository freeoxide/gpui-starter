use std::io::Cursor;
use std::path::Path;
use std::time::Duration;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::TrackType;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{get_codecs, get_probe};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AudioError {
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("no decodable audio track")]
    NoAudioTrack,
    #[error("no output device")]
    NoOutputDevice,
    #[error("device unsupported: {0}")]
    UnsupportedDevice(String),
    #[error("output failed: {0}")]
    Output(String),
    #[error("playback unavailable on this platform")]
    Unavailable,
}

/// Fully decoded, normalized audio: interleaved f32 in ±1.0, 1 or 2 channels.
#[derive(Debug)]
pub struct DecodedAudio {
    source_name: String,
    sample_rate: u32,
    channels: usize,
    samples: Vec<f32>,
}

impl DecodedAudio {
    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1)
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.frames() as f32 / self.sample_rate as f32)
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
}

/// Synchronous decode. `name` supplies the extension hint (".wav" routes to
/// hound, everything else to symphonia). Runs on the caller's thread.
pub fn decode_bytes(name: &str, bytes: Vec<u8>) -> Result<DecodedAudio, AudioError> {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let (mut samples, sample_rate, mut channels) = match ext.as_deref() {
        Some("wav") => decode_wav(&bytes)?,
        _ => decode_symphonia(bytes, ext)?,
    };
    adapt_channels(&mut samples, &mut channels)?;
    if samples.is_empty() {
        return Err(AudioError::NoAudioTrack);
    }
    Ok(DecodedAudio {
        source_name: name.to_string(),
        sample_rate,
        channels,
        samples,
    })
}

type DecodedParts = (Vec<f32>, u32, usize);

fn decode_err(err: impl std::error::Error) -> AudioError {
    AudioError::Decode(err.to_string())
}

fn decode_wav(bytes: &[u8]) -> Result<DecodedParts, AudioError> {
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(decode_err)?;
    let spec = reader.spec();
    let normalized = |den: f32| move |s: Result<i32, hound::Error>| s.map(|v| v as f32 / den);
    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .map(|s| s.map(|v| v.clamp(-1., 1.)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(decode_err)?,
        (hound::SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / 32_768.))
            .collect::<Result<Vec<_>, _>>()
            .map_err(decode_err)?,
        (hound::SampleFormat::Int, 24) => reader
            .samples::<i32>()
            .map(normalized(8_388_608.))
            .collect::<Result<Vec<_>, _>>()
            .map_err(decode_err)?,
        (hound::SampleFormat::Int, 32) => reader
            .samples::<i32>()
            .map(normalized(2_147_483_648.))
            .collect::<Result<Vec<_>, _>>()
            .map_err(decode_err)?,
        _ => {
            return Err(AudioError::Decode(
                "unsupported wav sample format".to_string(),
            ));
        }
    };
    Ok((samples, spec.sample_rate, spec.channels as usize))
}

fn map_symphonia_err(err: SymphError) -> AudioError {
    AudioError::Decode(err.to_string())
}

fn is_unexpected_eof(err: &SymphError) -> bool {
    matches!(err, SymphError::IoError(e) if e.kind() == std::io::ErrorKind::UnexpectedEof)
}

fn decode_symphonia(bytes: Vec<u8>, ext: Option<String>) -> Result<DecodedParts, AudioError> {
    let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = ext.as_deref() {
        hint.with_extension(ext);
    }
    let mut format = get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(map_symphonia_err)?;
    let (track_id, fallback_rate, fallback_channels, mut decoder) = {
        let track = format
            .default_track(TrackType::Audio)
            .ok_or(AudioError::NoAudioTrack)?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or(AudioError::NoAudioTrack)?;
        let decoder = get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(map_symphonia_err)?;
        (
            track.id,
            params.sample_rate,
            params.channels.as_ref().map(|c| c.count()),
            decoder,
        )
    };
    let mut samples = Vec::new();
    let mut block = Vec::new();
    let mut rate: Option<u32> = None;
    let mut channels: Option<usize> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(err) if is_unexpected_eof(&err) => break,
            Err(err) => return Err(map_symphonia_err(err)),
        };
        if packet.track_id != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(err) if is_unexpected_eof(&err) => break,
            Err(err) => return Err(map_symphonia_err(err)),
        };
        if rate.is_none() {
            let spec = decoded.spec();
            rate = Some(spec.rate());
            channels = Some(spec.channels().count());
        }
        decoded.copy_to_vec_interleaved::<f32>(&mut block);
        samples.append(&mut block);
    }
    let rate = rate.or(fallback_rate).ok_or(AudioError::NoAudioTrack)?;
    let channels = channels
        .or(fallback_channels)
        .ok_or(AudioError::NoAudioTrack)?;
    for sample in &mut samples {
        *sample = sample.clamp(-1., 1.);
    }
    Ok((samples, rate, channels))
}

fn adapt_channels(samples: &mut Vec<f32>, channels: &mut usize) -> Result<(), AudioError> {
    if *channels == 0 {
        return Err(AudioError::NoAudioTrack);
    }
    if *channels <= 2 {
        return Ok(());
    }
    let source = *channels;
    let left = source / 2;
    let right = source - left;
    let frames = samples.len() / source;
    let mut mixed = Vec::with_capacity(frames * 2);
    for frame in 0..frames {
        let chunk = &samples[frame * source..(frame + 1) * source];
        let l = chunk[..left].iter().sum::<f32>() / left as f32;
        let r = chunk[left..].iter().sum::<f32>() / right as f32;
        mixed.push(l.clamp(-1., 1.));
        mixed.push(r.clamp(-1., 1.));
    }
    *samples = mixed;
    *channels = 2;
    Ok(())
}

#[cfg(test)]
#[path = "decode.test.rs"]
mod decode_test;
