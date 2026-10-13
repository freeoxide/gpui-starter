use std::sync::Arc;

use cpal::SampleFormat;
use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};

use super::decode::AudioError;
use super::engine::OutputDevice;
use super::transport::SharedTransport;
use crate::ui::waveform::SampleProducer;

pub(crate) fn enumerate() -> Vec<OutputDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_output_device()
        .map(|device| device.to_string());
    let Ok(devices) = host.output_devices() else {
        return Vec::new();
    };
    devices
        .map(|device| {
            let name = device.to_string();
            OutputDevice {
                is_default: default_name.as_deref() == Some(name.as_str()),
                name,
            }
        })
        .collect()
}

#[derive(Clone)]
pub(crate) struct StreamPlan {
    pub config: cpal::StreamConfig,
    pub sample_rate: u32,
    pub channels: u16,
}

pub(crate) fn pick_rate(min: u32, max: u32, src: u32) -> u32 {
    if src >= min && src <= max {
        src
    } else if src < min {
        min
    } else {
        max
    }
}

/// Channel preference over f32 config ranges: exactly 2, else any >= 2
/// (clamped to 2 by the caller), else mono. Returns the range index.
pub(crate) fn pick_channels_index(channels: &[u16]) -> Option<usize> {
    channels
        .iter()
        .position(|count| *count == 2)
        .or_else(|| channels.iter().position(|count| *count >= 2))
        .or_else(|| channels.iter().position(|count| *count == 1))
}

pub(crate) fn plan_stream(
    device: &cpal::Device,
    source_rate: u32,
) -> Result<StreamPlan, AudioError> {
    let all = device
        .supported_output_configs()
        .map_err(|err| AudioError::UnsupportedDevice(err.to_string()))?
        .collect::<Vec<_>>();
    if all.is_empty() {
        return Err(AudioError::NoOutputDevice);
    }
    let f32_ranges = all
        .iter()
        .filter(|range| matches!(range.sample_format(), SampleFormat::F32))
        .collect::<Vec<_>>();
    let channels: Vec<u16> = f32_ranges.iter().map(|range| range.channels()).collect();
    let index = pick_channels_index(&channels)
        .ok_or_else(|| AudioError::UnsupportedDevice("no f32 output config".into()))?;
    let range = f32_ranges[index];
    let rate = pick_rate(
        range.min_sample_rate(),
        range.max_sample_rate(),
        source_rate,
    );
    let config = range
        .try_with_sample_rate(rate)
        .ok_or_else(|| AudioError::UnsupportedDevice(format!("sample rate {rate} unsupported")))?;
    let channels = range.channels().min(2);
    Ok(StreamPlan {
        config: cpal::StreamConfig {
            channels,
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        },
        sample_rate: config.sample_rate(),
        channels,
    })
}

pub(crate) fn fill_buffer(
    src: &[f32],
    channels: usize,
    transport: &SharedTransport,
    out: &mut [f32],
    tap: &mut Vec<f32>,
) -> bool {
    let channels = channels.max(1);
    let cursor = transport.cursor_frames() as usize;
    let total = src.len() / channels;
    let frames_out = out.len() / channels;
    let avail = total.saturating_sub(cursor);
    let take = frames_out.min(avail);
    let volume = transport.volume();
    let start = cursor * channels;
    for i in 0..take * channels {
        out[i] = src[start + i] * volume;
    }
    out[take * channels..].fill(0.);
    tap.clear();
    tap.extend_from_slice(&out[..take * channels]);
    transport.set_cursor((cursor + take) as u64);
    take < frames_out
}

pub(crate) struct OutputSession {
    stream: cpal::Stream,
}

impl OutputSession {
    pub(crate) fn play(&self) -> Result<(), AudioError> {
        self.stream
            .play()
            .map_err(|err| AudioError::Output(err.to_string()))
    }

    pub(crate) fn pause(&self) -> Result<(), AudioError> {
        self.stream
            .pause()
            .map_err(|err| AudioError::Output(err.to_string()))
    }
}

/// Resolve the device for `wanted` (by name), falling back to the host
/// default. Runs the host's device scan; call from a background task.
pub(crate) fn find_output_device(wanted: Option<&str>) -> Result<cpal::Device, AudioError> {
    let host = cpal::default_host();
    let device = match wanted {
        Some(name) => host
            .output_devices()
            .map_err(|err| AudioError::UnsupportedDevice(err.to_string()))?
            .find(|device| device.to_string() == name)
            .or_else(|| host.default_output_device()),
        None => host.default_output_device(),
    };
    device.ok_or(AudioError::NoOutputDevice)
}

pub(crate) fn build_session(
    device: &cpal::Device,
    render: Arc<Vec<f32>>,
    render_channels: usize,
    plan: StreamPlan,
    transport: Arc<SharedTransport>,
    mut producer: SampleProducer,
) -> Result<OutputSession, AudioError> {
    let mut tap: Vec<f32> = Vec::new();
    let stream = device
        .build_output_stream::<f32, _, _>(
            plan.config,
            move |data: &mut [f32], _| {
                if fill_buffer(&render, render_channels, &transport, data, &mut tap) {
                    transport.set_playing(false);
                    transport.set_eof();
                }
                producer.push_interleaved(&tap);
            },
            |err| tracing::warn!(target: "gpui_starter::audio", error = %err, "cpal stream error"),
            None,
        )
        .map_err(|err| AudioError::Output(err.to_string()))?;
    Ok(OutputSession { stream })
}

pub(crate) fn resample_linear(src: &[f32], channels: usize, from: u32, to: u32) -> Vec<f32> {
    if from == to || src.is_empty() {
        return src.to_vec();
    }
    let channels = channels.max(1);
    let frames_in = src.len() / channels;
    let frames_out = (frames_in as f64 * to as f64 / from as f64).round() as usize;
    let mut out = Vec::with_capacity(frames_out * channels);
    for frame in 0..frames_out {
        let t = frame as f64 * from as f64 / to as f64;
        let base = t.floor() as usize;
        let frac = (t - t.floor()) as f32;
        let a = base.min(frames_in - 1);
        let b = (base + 1).min(frames_in - 1);
        for channel in 0..channels {
            let left = src[a * channels + channel];
            let right = src[b * channels + channel];
            out.push(left + (right - left) * frac);
        }
    }
    out
}

pub(crate) fn adapt_to_channels(src: &[f32], channels: usize, out_channels: usize) -> Vec<f32> {
    if channels == out_channels {
        return src.to_vec();
    }
    let channels = channels.max(1);
    let frames = src.len() / channels;
    let mut out = Vec::with_capacity(frames * out_channels);
    for frame in 0..frames {
        let chunk = &src[frame * channels..(frame + 1) * channels];
        match (channels, out_channels) {
            (1, 2) => {
                out.push(chunk[0]);
                out.push(chunk[0]);
            }
            (2, 1) => out.push((chunk[0] + chunk[1]) * 0.5),
            _ => out.extend_from_slice(chunk),
        }
    }
    out
}

#[cfg(test)]
#[path = "output.test.rs"]
mod output_test;
