use std::io::Cursor;

use hound::WavReader;

fn zero_crossings(samples: &[i16]) -> usize {
    samples
        .windows(2)
        .filter(|pair| pair[0].signum() != pair[1].signum())
        .count()
}

#[test]
fn sweep_wav_matches_spec_and_bounds() {
    let bytes = super::sweep_wav(1);
    let mut reader = WavReader::new(Cursor::new(&bytes[..])).unwrap();
    assert_eq!(reader.spec().sample_rate, 44_100);
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.duration(), 44_100);
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
    assert_eq!(samples.len(), 88_200);
    assert!(samples.iter().all(|s| (s.unsigned_abs() as u32) < 19_660));
}

#[test]
fn sweep_wav_rises_from_bass_to_treble() {
    let bytes = super::sweep_wav(2);
    let samples: Vec<i16> = WavReader::new(Cursor::new(&bytes[..]))
        .unwrap()
        .samples()
        .map(|s| s.unwrap())
        .collect();
    let mono: Vec<i16> = samples.iter().step_by(2).copied().collect();
    assert!(zero_crossings(&mono[..4410]) < 10);
    assert!(zero_crossings(&mono[mono.len() - 4410..]) > 100);
}

#[test]
fn chords_wav_matches_spec_and_stays_audible() {
    let bytes = super::chords_wav(4);
    let mut reader = WavReader::new(Cursor::new(&bytes[..])).unwrap();
    assert_eq!(reader.spec().sample_rate, 44_100);
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.duration(), 4 * 44_100);
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
    assert!(samples.iter().all(|s| (s.unsigned_abs() as u32) < 19_660));
    for block in samples.chunks(2 * 44_100) {
        assert!(
            block.iter().any(|s| s.unsigned_abs() > 500),
            "a 1-second block is silent"
        );
    }
}
