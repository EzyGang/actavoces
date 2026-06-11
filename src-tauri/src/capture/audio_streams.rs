use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, StreamTrait};

use crate::capture::audio::audio_devices::resolve_capture_device;
use crate::domain::types::CaptureSource;

pub(crate) struct CapturedSource {
    pub(crate) source: CaptureSource,
    pub(crate) _stream: cpal::Stream,
    pub(crate) samples: Arc<Mutex<Vec<i16>>>,
    pub(crate) stream_errors: Arc<Mutex<Vec<String>>>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

pub(crate) trait ToI16Sample {
    fn to_i16_sample(&self) -> i16;
}

pub(crate) fn start_native_source(
    host: &cpal::Host,
    source: CaptureSource,
    configured_name: &str,
) -> Result<CapturedSource, String> {
    let capture_device = resolve_capture_device(host, source, configured_name)?;
    let device = capture_device.device;
    let config = capture_device.config;
    let stream_config = config.config();
    let sample_rate = stream_config.sample_rate;
    let channels = stream_config.channels;
    let samples = Arc::new(Mutex::new(Vec::<i16>::new()));
    let stream_errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let stream = build_input_stream(
        &device,
        config.sample_format(),
        stream_config,
        &samples,
        &stream_errors,
    )?;

    stream
        .play()
        .map_err(|error| format!("Unable to start input stream: {error}"))?;

    Ok(CapturedSource {
        source,
        _stream: stream,
        samples,
        stream_errors,
        sample_rate,
        channels,
    })
}

pub(crate) fn build_input_stream(
    device: &cpal::Device,
    sample_format: cpal::SampleFormat,
    stream_config: cpal::StreamConfig,
    samples: &Arc<Mutex<Vec<i16>>>,
    stream_errors: &Arc<Mutex<Vec<String>>>,
) -> Result<cpal::Stream, String> {
    match sample_format {
        cpal::SampleFormat::F32 => {
            build_typed_input_stream::<f32>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::F64 => {
            build_typed_input_stream::<f64>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::I8 => {
            build_typed_input_stream::<i8>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::I16 => {
            build_typed_input_stream::<i16>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::I32 => {
            build_typed_input_stream::<i32>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::I64 => {
            build_typed_input_stream::<i64>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::U8 => {
            build_typed_input_stream::<u8>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::U16 => {
            build_typed_input_stream::<u16>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::U32 => {
            build_typed_input_stream::<u32>(device, stream_config, samples, stream_errors)
        }
        cpal::SampleFormat::U64 => {
            build_typed_input_stream::<u64>(device, stream_config, samples, stream_errors)
        }
        format => Err(format!("Unsupported input sample format: {format:?}")),
    }
}

pub(crate) fn build_typed_input_stream<T>(
    device: &cpal::Device,
    stream_config: cpal::StreamConfig,
    samples: &Arc<Mutex<Vec<i16>>>,
    stream_errors: &Arc<Mutex<Vec<String>>>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + ToI16Sample,
{
    let samples = Arc::clone(samples);
    let stream_errors = Arc::clone(stream_errors);

    device
        .build_input_stream(
            stream_config,
            move |data: &[T], _info| {
                if let Ok(mut captured_samples) = samples.lock() {
                    for sample in data {
                        captured_samples.push(sample.to_i16_sample());
                    }
                }
            },
            move |error| {
                if let Ok(mut errors) = stream_errors.lock() {
                    errors.push(error.to_string());
                }
            },
            None,
        )
        .map_err(|error| format!("Unable to build input stream: {error}"))
}

impl ToI16Sample for f32 {
    fn to_i16_sample(&self) -> i16 {
        (self.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
    }
}

impl ToI16Sample for f64 {
    fn to_i16_sample(&self) -> i16 {
        (self.clamp(-1.0, 1.0) * i16::MAX as f64) as i16
    }
}

impl ToI16Sample for i8 {
    fn to_i16_sample(&self) -> i16 {
        (*self as i16) << 8
    }
}

impl ToI16Sample for i16 {
    fn to_i16_sample(&self) -> i16 {
        *self
    }
}

impl ToI16Sample for i32 {
    fn to_i16_sample(&self) -> i16 {
        (*self >> 16) as i16
    }
}

impl ToI16Sample for i64 {
    fn to_i16_sample(&self) -> i16 {
        (*self >> 48) as i16
    }
}

impl ToI16Sample for u8 {
    fn to_i16_sample(&self) -> i16 {
        ((*self as i16) - 128) << 8
    }
}

impl ToI16Sample for u16 {
    fn to_i16_sample(&self) -> i16 {
        (*self as i32 - 32_768) as i16
    }
}

impl ToI16Sample for u32 {
    fn to_i16_sample(&self) -> i16 {
        ((*self >> 16) as i32 - 32_768) as i16
    }
}

impl ToI16Sample for u64 {
    fn to_i16_sample(&self) -> i16 {
        ((*self >> 48) as i32 - 32_768) as i16
    }
}
