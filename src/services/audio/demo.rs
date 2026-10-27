use std::f64::consts::TAU;
use std::io::Cursor;

use hound::{SampleFormat, WavSpec, WavWriter};

const DEMO_RATE: u32 = 44_100;

pub(crate) fn sweep_wav(seconds: u32) -> Vec<u8> {
    let seconds = seconds.max(1);
    let frames = seconds as usize * DEMO_RATE as usize;
    let mut bytes = Vec::new();
    {
        let mut writer = WavWriter::new(
            Cursor::new(&mut bytes),
            WavSpec {
                channels: 2,
                sample_rate: DEMO_RATE,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            },
        )
        .expect("in-memory wav write cannot fail");
        let mut phase = 0f64;
        for frame in 0..frames {
            let t = frame as f64 / DEMO_RATE as f64;
            let freq = 20. * 1000f64.powf(t / seconds as f64);
            phase += freq / DEMO_RATE as f64;
            let sample = (0.5 * (TAU * phase).sin() * i16::MAX as f64) as i16;
            writer
                .write_sample(sample)
                .expect("in-memory wav write cannot fail");
            writer
                .write_sample(sample)
                .expect("in-memory wav write cannot fail");
        }
        writer.finalize().expect("in-memory wav write cannot fail");
    }
    bytes
}

pub(crate) fn chords_wav(seconds: u32) -> Vec<u8> {
    const BLOCK_SECS: usize = 2;
    const CHORDS: [[f64; 3]; 4] = [
        [261.63, 329.63, 392.00],
        [220.00, 261.63, 329.63],
        [174.61, 220.00, 261.63],
        [196.00, 246.94, 293.66],
    ];
    let frames = seconds as usize * DEMO_RATE as usize;
    let mut bytes = Vec::new();
    {
        let mut writer = WavWriter::new(
            Cursor::new(&mut bytes),
            WavSpec {
                channels: 2,
                sample_rate: DEMO_RATE,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            },
        )
        .expect("in-memory wav write cannot fail");
        for frame in 0..frames {
            let t = frame as f64 / DEMO_RATE as f64;
            let chord = CHORDS[(frame / (BLOCK_SECS * DEMO_RATE as usize)) % CHORDS.len()];
            let mixed =
                0.4 * chord.iter().map(|f| (TAU * f * t).sin()).sum::<f64>() / chord.len() as f64;
            let sample = (mixed * i16::MAX as f64) as i16;
            writer
                .write_sample(sample)
                .expect("in-memory wav write cannot fail");
            writer
                .write_sample(sample)
                .expect("in-memory wav write cannot fail");
        }
        writer.finalize().expect("in-memory wav write cannot fail");
    }
    bytes
}

#[cfg(test)]
#[path = "demo.test.rs"]
mod demo_test;
