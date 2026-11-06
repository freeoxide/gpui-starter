use std::io::Cursor;

use hound::{SampleFormat, WavSpec, WavWriter};

use super::{AudioError, DecodedAudio, decode_bytes};
use crate::audio::demo;

fn mono_wav(value: i16) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut writer = WavWriter::new(
        Cursor::new(&mut bytes),
        WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    )
    .unwrap();
    writer.write_sample(value).unwrap();
    writer.finalize().unwrap();
    bytes
}

fn quad_wav(values: [i16; 4]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut writer = WavWriter::new(
        Cursor::new(&mut bytes),
        WavSpec {
            channels: 4,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    )
    .unwrap();
    for value in values {
        writer.write_sample(value).unwrap();
    }
    writer.finalize().unwrap();
    bytes
}

fn float_wav(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut writer = WavWriter::new(
        Cursor::new(&mut bytes),
        WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    )
    .unwrap();
    for value in values {
        writer.write_sample(*value).unwrap();
    }
    writer.finalize().unwrap();
    bytes
}

fn int_wav(value: i32, bits_per_sample: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut writer = WavWriter::new(
        Cursor::new(&mut bytes),
        WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample,
            sample_format: SampleFormat::Int,
        },
    )
    .unwrap();
    writer.write_sample(value).unwrap();
    writer.finalize().unwrap();
    bytes
}

#[test]
fn sweep_wav_roundtrips_through_hound() {
    let decoded = decode_bytes("sweep.wav", demo::sweep_wav(1)).unwrap();
    assert_eq!(decoded.sample_rate(), 44_100);
    assert_eq!(decoded.channels(), 2);
    assert_eq!(decoded.frames(), 44_100);
    assert_eq!(decoded.duration(), std::time::Duration::from_secs(1));
    assert_eq!(decoded.source_name(), "sweep.wav");
}

#[test]
fn i16_wav_normalizes_to_half() {
    let decoded = decode_bytes("mono.wav", mono_wav(16_384)).unwrap();
    assert_eq!(decoded.channels(), 1);
    assert_eq!(decoded.samples(), [0.5]);
}

#[test]
fn int24_wav_normalizes_full_scale_negative_to_minus_one() {
    let decoded = decode_bytes("i24.wav", int_wav(-8_388_608, 24)).unwrap();
    assert_eq!(decoded.samples(), [-1.0]);
}

#[test]
fn int32_wav_normalizes_full_scale_negative_to_minus_one() {
    let decoded = decode_bytes("i32.wav", int_wav(-2_147_483_648, 32)).unwrap();
    assert_eq!(decoded.samples(), [-1.0]);
}

#[test]
fn int8_wav_is_rejected() {
    let err = decode_bytes("i8.wav", int_wav(0, 8)).unwrap_err();
    assert!(matches!(err, AudioError::Decode(_)), "{err:?}");
}

#[test]
fn zero_sample_rate_wav_is_rejected() {
    let mut bytes = mono_wav(16_384);
    bytes[24..28].copy_from_slice(&0u32.to_le_bytes());
    let err = decode_bytes("zero.wav", bytes).unwrap_err();
    assert!(matches!(err, AudioError::Decode(_)), "{err:?}");
}

#[test]
fn wav_without_samples_has_no_track() {
    let mut bytes = Vec::new();
    let writer = WavWriter::new(
        Cursor::new(&mut bytes),
        WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    )
    .unwrap();
    writer.finalize().unwrap();
    let err = decode_bytes("empty.wav", bytes).unwrap_err();
    assert!(matches!(err, AudioError::NoAudioTrack), "{err:?}");
}

#[test]
fn float_wav_passes_through() {
    let decoded = decode_bytes("float.wav", float_wav(&[0.25, -0.75])).unwrap();
    assert_eq!(decoded.sample_rate(), 48_000);
    assert_eq!(decoded.samples(), [0.25, -0.75]);
}

#[test]
fn quad_wav_downmixes_to_stereo() {
    let decoded = decode_bytes("quad.wav", quad_wav([16_384, 16_384, -16_384, 16_384])).unwrap();
    assert_eq!(decoded.channels(), 2);
    assert_eq!(decoded.frames(), 1);
    assert_eq!(decoded.samples(), [0.5, 0.0]);
}

#[test]
fn garbage_bytes_fail_to_decode() {
    let err = decode_bytes("x.wav", b"not audio".to_vec()).unwrap_err();
    assert!(matches!(err, AudioError::Decode(_)), "{err:?}");
}

#[test]
fn truncated_wav_fails_to_decode() {
    let full = demo::sweep_wav(1);
    let err = decode_bytes("x.wav", full[..44].to_vec()).unwrap_err();
    assert!(matches!(err, AudioError::Decode(_)), "{err:?}");
}

#[test]
fn symphonia_path_decodes_wav_like_hound() {
    let (samples, rate, channels) =
        super::decode_symphonia(demo::sweep_wav(1), Some("wav".to_string())).unwrap();
    assert_eq!(rate, 44_100);
    assert_eq!(channels, 2);
    assert_eq!(samples.len(), 88_200);
}

#[test]
fn symphonia_tolerates_a_mid_stream_eof() {
    let full = demo::sweep_wav(1);
    // Cut inside the declared data chunk: packets read so far decode, the
    // eof read must break the loop instead of failing the whole decode.
    let truncated = full[..44 + 4608 * 4].to_vec();
    let (samples, rate, channels) =
        super::decode_symphonia(truncated, Some("wav".to_string())).unwrap();
    assert_eq!((rate, channels), (44_100, 2));
    assert!(!samples.is_empty());
    assert!(samples.len() < 88_200, "decoded {}", samples.len());
}

#[test]
fn symphonia_path_rejects_garbage_without_hint() {
    let err = super::decode_symphonia(b"not audio".to_vec(), None).unwrap_err();
    assert!(matches!(err, AudioError::Decode(_)), "{err:?}");
}

#[test]
fn compressed_extensions_route_to_symphonia() {
    for name in ["x.mp3", "x.flac"] {
        let err = decode_bytes(name, b"not audio".to_vec()).unwrap_err();
        assert!(matches!(err, AudioError::Decode(_)), "{name}: {err:?}");
    }
}

#[test]
fn decoded_parts_report_their_stream_shape() {
    let decoded = decode_bytes("sweep.wav", demo::sweep_wav(1)).unwrap();
    let round: DecodedAudio = decode_bytes("sweep.wav", demo::sweep_wav(1)).unwrap();
    assert_eq!(decoded.duration(), round.duration());
    assert_eq!(decoded.frames(), round.frames());
}
