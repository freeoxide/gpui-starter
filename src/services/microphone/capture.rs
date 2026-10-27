use std::sync::Arc;
use std::sync::atomic::AtomicI32;

use cpal::SizedSample;
use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};

use super::controller::gain_multiplier;
use super::meter::LevelMeter;
use super::types::InputDeviceInfo;
use crate::ui::waveform::SampleProducer;

pub(crate) fn enumerate() -> Result<Vec<InputDeviceInfo>, String> {
    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|device| device.id().ok())
        .map(|id| id.to_string());
    let devices = host
        .input_devices()
        .map_err(|err| err.to_string())?
        .filter_map(|device| {
            let id = device.id().ok()?.to_string();
            let config = device.default_input_config().ok()?;
            if config.channels() == 0 {
                return None;
            }
            let name = device
                .description()
                .map(|description| description.name().to_string())
                .unwrap_or_else(|_| device.to_string());
            let is_default = default_id.as_deref() == Some(id.as_str());
            Some(InputDeviceInfo {
                id,
                name,
                is_default,
                channels: config.channels(),
                sample_rate: config.sample_rate(),
                sample_format: format!("{:?}", config.sample_format()),
            })
        })
        .collect();
    Ok(devices)
}

pub(crate) enum SessionCommand {
    Stop,
}

pub(crate) enum SessionEvent {
    Started,
    Failed(String),
    Stopped,
}

/// Resolution order wanted-id, then default, then first.
pub(crate) fn pick_device<'a>(
    mut devices: impl Iterator<Item = &'a InputDeviceInfo>,
    wanted: Option<&str>,
) -> Option<InputDeviceInfo> {
    let mut first = None;
    let mut default = None;
    for device in &mut devices {
        if wanted == Some(device.id.as_str()) {
            return Some(device.clone());
        }
        first.get_or_insert_with(|| device.clone());
        if device.is_default {
            default.get_or_insert_with(|| device.clone());
        }
    }
    default.or(first)
}

trait ToF32 {
    fn to_f32_(&self) -> f32;
}

impl ToF32 for f32 {
    fn to_f32_(&self) -> f32 {
        *self
    }
}

impl ToF32 for i16 {
    fn to_f32_(&self) -> f32 {
        *self as f32 / 32768.
    }
}

impl ToF32 for u16 {
    fn to_f32_(&self) -> f32 {
        (*self as f32 - 32768.) / 32768.
    }
}

impl ToF32 for i32 {
    fn to_f32_(&self) -> f32 {
        *self as f32 / 2147483648.
    }
}

fn make_callback<T: SizedSample + Copy + ToF32 + Send + 'static>(
    mut producer: SampleProducer,
    meter: Arc<LevelMeter>,
    gain_q8: Arc<AtomicI32>,
    dev_channels: usize,
    channels: usize,
) -> impl FnMut(&[T], &cpal::InputCallbackInfo) + Send + 'static {
    move |data: &[T], _: &cpal::InputCallbackInfo| {
        convert_chunk(
            data,
            &mut producer,
            &meter,
            &gain_q8,
            dev_channels,
            channels,
        );
    }
}

fn convert_chunk<T: ToF32>(
    data: &[T],
    producer: &mut SampleProducer,
    meter: &LevelMeter,
    gain_q8: &AtomicI32,
    dev_channels: usize,
    channels: usize,
) {
    let gain = gain_multiplier(gain_q8.load(std::sync::atomic::Ordering::Relaxed));
    let mut buf = [0f32; 1024];
    let mut len = 0;
    let mut peak = 0f32;
    let mut sum_sq = 0f32;
    let mut n = 0usize;
    for frame in data.chunks(dev_channels) {
        for ch in 0..channels {
            let raw = frame.get(ch).map_or(0., |value| value.to_f32_() * gain);
            let sample = if raw.is_finite() { raw } else { 0. };
            peak = peak.max(sample.abs());
            sum_sq += sample * sample;
            n += 1;
            buf[len] = sample;
            len += 1;
            if len == buf.len() {
                producer.push_interleaved(&buf);
                len = 0;
            }
        }
    }
    if len > 0 {
        producer.push_interleaved(&buf[..len]);
    }
    let rms = if n == 0 {
        0.
    } else {
        (sum_sq / n as f32).sqrt()
    };
    meter.update(rms, peak);
}

fn build_and_play(
    device_id: &str,
    producer: SampleProducer,
    meter: Arc<LevelMeter>,
    gain_q8: Arc<AtomicI32>,
    ev_tx: flume::Sender<SessionEvent>,
) -> Result<cpal::Stream, String> {
    let devices = enumerate()?;
    let info = devices
        .into_iter()
        .find(|device| device.id == device_id)
        .ok_or_else(|| "device disappeared".to_string())?;
    let host = cpal::default_host();
    let device = host
        .input_devices()
        .map_err(|err| err.to_string())?
        .find(|candidate| candidate.id().map(|id| id.to_string()) == Ok(info.id.clone()))
        .ok_or_else(|| "device disappeared".to_string())?;
    let config = device
        .default_input_config()
        .map_err(|err| err.to_string())?;
    let dev_channels = config.channels() as usize;
    let wave_channels = dev_channels.clamp(1, 2);
    let error_tx = ev_tx.clone();
    let error_callback = move |err: cpal::Error| {
        tracing::warn!(target: "gpui_starter::microphone", error = %err, "input stream error");
        let _ = error_tx.send(SessionEvent::Failed(err.to_string()));
    };
    let stream_config = config.config();
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            stream_config,
            make_callback::<f32>(producer, meter, gain_q8, dev_channels, wave_channels),
            error_callback,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            stream_config,
            make_callback::<i16>(producer, meter, gain_q8, dev_channels, wave_channels),
            error_callback,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            stream_config,
            make_callback::<u16>(producer, meter, gain_q8, dev_channels, wave_channels),
            error_callback,
            None,
        ),
        cpal::SampleFormat::I32 => device.build_input_stream(
            stream_config,
            make_callback::<i32>(producer, meter, gain_q8, dev_channels, wave_channels),
            error_callback,
            None,
        ),
        other => {
            return Err(format!(
                "{}: {other:?}",
                crate::i18n::localize("microphone_unsupported_format")
            ));
        }
    }
    .map_err(|err| err.to_string())?;
    stream.play().map_err(|err| err.to_string())?;
    Ok(stream)
}

/// Cancellation-safe: dropping the owning `Task` drops this future at its
/// await, dropping the stream (cpal streams pause on drop).
pub(crate) async fn run_session(
    device_id: String,
    producer: SampleProducer,
    meter: Arc<LevelMeter>,
    gain_q8: Arc<AtomicI32>,
    cmd_rx: flume::Receiver<SessionCommand>,
    ev_tx: flume::Sender<SessionEvent>,
) {
    match build_and_play(&device_id, producer, meter, gain_q8, ev_tx.clone()) {
        Ok(stream) => {
            let _ = ev_tx.send(SessionEvent::Started);
            // Parks until the one command arrives or the sender drops.
            let _ = cmd_rx.recv_async().await;
            drop(stream);
            let _ = ev_tx.send(SessionEvent::Stopped);
        }
        Err(err) => {
            let _ = ev_tx.send(SessionEvent::Failed(err));
        }
    }
}

#[cfg(test)]
#[path = "capture.test.rs"]
mod capture_test;
