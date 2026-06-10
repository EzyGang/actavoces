use std::collections::HashMap;
#[cfg(test)]
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;

use crate::artifacts::capture_artifacts_with_readiness;
#[cfg(test)]
use crate::artifacts::write_test_wav_file;
use crate::domain::types::*;
pub(crate) trait AudioCaptureBackend {
    fn start(&mut self, recording_id: &str, settings: &AppSettings) -> Result<(), String>;

    fn stop(
        &mut self,
        recording_id: &str,
        artifact_directory: &Path,
    ) -> Result<CaptureResult, String>;
}

#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct FileAudioCaptureBackend {
    pub(crate) active_recordings: HashSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CaptureResult {
    pub(crate) artifacts: Vec<Artifact>,
    pub(crate) errors: Vec<CaptureError>,
}

#[cfg(test)]
impl AudioCaptureBackend for FileAudioCaptureBackend {
    fn start(&mut self, recording_id: &str, _settings: &AppSettings) -> Result<(), String> {
        self.active_recordings.insert(recording_id.to_owned());

        Ok(())
    }

    fn stop(
        &mut self,
        recording_id: &str,
        artifact_directory: &Path,
    ) -> Result<CaptureResult, String> {
        if !self.active_recordings.remove(recording_id) {
            return Err("Capture backend does not have an active session".to_owned());
        }

        fs::create_dir_all(artifact_directory).map_err(|error| error.to_string())?;
        write_test_wav_file(&artifact_directory.join("recording.wav"), 440)?;
        write_test_wav_file(&artifact_directory.join("microphone.wav"), 880)?;
        write_capture_metadata(
            artifact_directory,
            "file",
            &[
                CaptureFileMetadata::ready(CaptureSource::Microphone, "microphone.wav"),
                CaptureFileMetadata::ready(CaptureSource::System, "recording.wav"),
            ],
        )?;
        fs::write(
            artifact_directory.join("job-log.jsonl"),
            "{\"stage\":\"recording\",\"status\":\"complete\",\"message\":\"capture stopped\"}\n",
        )
        .map_err(|error| error.to_string())?;

        Ok(CaptureResult {
            artifacts: capture_artifacts_with_readiness(artifact_directory, true, true),
            errors: Vec::new(),
        })
    }
}

#[derive(Default)]
pub(crate) struct NativeAudioCaptureBackend {
    pub(crate) active_recordings: HashMap<String, NativeCaptureSession>,
}

impl std::fmt::Debug for NativeAudioCaptureBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeAudioCaptureBackend")
            .field("active_recordings", &self.active_recordings.keys())
            .finish()
    }
}

pub(crate) struct NativeCaptureSession {
    pub(crate) microphone: Option<CapturedSource>,
    pub(crate) system: Option<CapturedSource>,
    pub(crate) errors: Vec<CaptureError>,
}

pub(crate) struct CapturedSource {
    pub(crate) source: CaptureSource,
    pub(crate) _stream: cpal::Stream,
    pub(crate) samples: Arc<Mutex<Vec<i16>>>,
    pub(crate) stream_errors: Arc<Mutex<Vec<String>>>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

pub(crate) struct CaptureDevice {
    pub(crate) device: cpal::Device,
    pub(crate) config: cpal::SupportedStreamConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureFileMetadata {
    pub(crate) source: CaptureSource,
    pub(crate) path: String,
    pub(crate) ready: bool,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) channels: Option<u16>,
    pub(crate) frames: usize,
}

impl CaptureFileMetadata {
    #[cfg(test)]
    fn ready(source: CaptureSource, path: &str) -> Self {
        Self {
            source,
            path: path.to_owned(),
            ready: true,
            sample_rate: None,
            channels: None,
            frames: 0,
        }
    }
}

impl AudioCaptureBackend for NativeAudioCaptureBackend {
    fn start(&mut self, recording_id: &str, settings: &AppSettings) -> Result<(), String> {
        if self.active_recordings.contains_key(recording_id) {
            return Err("Capture backend already has an active session".to_owned());
        }

        let host = cpal::default_host();
        let mut errors = Vec::new();
        let microphone = match start_native_source(
            &host,
            CaptureSource::Microphone,
            &settings.microphone_device,
        ) {
            Ok(source) => Some(source),
            Err(error) => {
                errors.push(CaptureError {
                    source: CaptureSource::Microphone,
                    message: error,
                });
                None
            }
        };
        let system = match start_native_source(
            &host,
            CaptureSource::System,
            &settings.system_audio_source,
        ) {
            Ok(source) => Some(source),
            Err(error) => {
                errors.push(CaptureError {
                    source: CaptureSource::System,
                    message: error,
                });
                None
            }
        };

        if microphone.is_none() && system.is_none() {
            return Err(capture_errors_message(&errors));
        }

        self.active_recordings.insert(
            recording_id.to_owned(),
            NativeCaptureSession {
                microphone,
                system,
                errors,
            },
        );

        Ok(())
    }

    fn stop(
        &mut self,
        recording_id: &str,
        artifact_directory: &Path,
    ) -> Result<CaptureResult, String> {
        let session = self
            .active_recordings
            .remove(recording_id)
            .ok_or_else(|| "Capture backend does not have an active session".to_owned())?;

        fs::create_dir_all(artifact_directory).map_err(|error| error.to_string())?;

        let mut errors = session.errors;
        let microphone = finalize_native_source(&session.microphone)?;
        let system = finalize_native_source(&session.system)?;

        append_stream_errors(&mut errors, &session.microphone);
        append_stream_errors(&mut errors, &session.system);
        if let Some(microphone) = microphone.as_ref() {
            write_pcm_wav_file(&artifact_directory.join("microphone.wav"), microphone)?;
        }
        write_mixed_recording(
            artifact_directory,
            microphone.as_ref(),
            system.as_ref(),
            "recording.wav",
        )?;
        write_job_log(artifact_directory, &errors)?;
        write_capture_metadata(
            artifact_directory,
            "native-cpal",
            &[
                metadata_for_source(
                    CaptureSource::Microphone,
                    "microphone.wav",
                    microphone.as_ref(),
                ),
                metadata_for_source(CaptureSource::System, "recording.wav", system.as_ref()),
            ],
        )?;

        Ok(CaptureResult {
            artifacts: capture_artifacts_with_readiness(
                artifact_directory,
                microphone.is_some() || system.is_some(),
                microphone.is_some(),
            ),
            errors,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FinalizedSource {
    pub(crate) samples: Vec<i16>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) frames: usize,
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

pub(crate) trait ToI16Sample {
    fn to_i16_sample(&self) -> i16;
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

pub(crate) fn resolve_capture_device(
    host: &cpal::Host,
    source: CaptureSource,
    configured_name: &str,
) -> Result<CaptureDevice, String> {
    let device = match source {
        CaptureSource::Microphone => resolve_microphone_device(host, configured_name)?,
        CaptureSource::System => resolve_system_device(host, configured_name)?,
    };
    let config = capture_config_for_device(&device, source)?;

    Ok(CaptureDevice { device, config })
}

pub(crate) fn resolve_microphone_device(
    host: &cpal::Host,
    configured_name: &str,
) -> Result<cpal::Device, String> {
    let configured_name = configured_name.trim();

    if is_default_microphone_name(configured_name) {
        return host
            .default_input_device()
            .ok_or_else(|| "No default microphone input device is available".to_owned());
    }

    resolve_named_device(host, configured_name, DeviceSearchMode::Input)
}

pub(crate) fn resolve_system_device(
    host: &cpal::Host,
    configured_name: &str,
) -> Result<cpal::Device, String> {
    let configured_name = configured_name.trim();

    if is_default_system_source_name(configured_name) {
        return resolve_default_system_device(host);
    }

    resolve_named_device(host, configured_name, DeviceSearchMode::System)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) fn resolve_default_system_device(host: &cpal::Host) -> Result<cpal::Device, String> {
    host.default_output_device().ok_or_else(|| {
        "No default output device is available for native system audio capture".to_owned()
    })
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd"
))]
pub(crate) fn resolve_default_system_device(host: &cpal::Host) -> Result<cpal::Device, String> {
    for device in host
        .input_devices()
        .map_err(|error| format!("Unable to list input devices: {error}"))?
    {
        if is_system_monitor_device_name(&device.to_string()) {
            return Ok(device);
        }
    }

    Err("System audio capture on Linux requires a PipeWire/PulseAudio monitor or loopback input device in Capture settings".to_owned())
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd"
)))]
pub(crate) fn resolve_default_system_device(_host: &cpal::Host) -> Result<cpal::Device, String> {
    Err("Native system audio capture is not supported on this platform".to_owned())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeviceSearchMode {
    Input,
    System,
}

pub(crate) fn resolve_named_device(
    host: &cpal::Host,
    configured_name: &str,
    mode: DeviceSearchMode,
) -> Result<cpal::Device, String> {
    let configured_name_lower = configured_name.to_lowercase();

    for device in host
        .input_devices()
        .map_err(|error| format!("Unable to list input devices: {error}"))?
    {
        let device_name = device.to_string();

        if device_name.to_lowercase().contains(&configured_name_lower)
            || (mode == DeviceSearchMode::System
                && is_monitor_search_name(&configured_name_lower)
                && is_system_monitor_device_name(&device_name))
        {
            return Ok(device);
        }
    }

    if mode == DeviceSearchMode::System {
        for device in host
            .output_devices()
            .map_err(|error| format!("Unable to list output devices: {error}"))?
        {
            if device
                .to_string()
                .to_lowercase()
                .contains(&configured_name_lower)
            {
                return Ok(device);
            }
        }
    }

    Err(format!("Audio device not found: {configured_name}"))
}

pub(crate) fn capture_config_for_device(
    device: &cpal::Device,
    source: CaptureSource,
) -> Result<cpal::SupportedStreamConfig, String> {
    if device.supports_input() {
        return device
            .default_input_config()
            .map_err(|error| format!("Unable to read input config: {error}"));
    }

    if source == CaptureSource::System && device.supports_output() {
        return device
            .default_output_config()
            .map_err(|error| format!("Unable to read output loopback config: {error}"));
    }

    Err(format!("Audio device cannot capture {source:?} audio"))
}

pub(crate) fn is_default_microphone_name(name: &str) -> bool {
    name.is_empty() || name == "Default microphone"
}

pub(crate) fn is_default_system_source_name(name: &str) -> bool {
    name.is_empty() || name == "Default system output"
}

pub(crate) fn is_system_monitor_device_name(name: &str) -> bool {
    let normalized = name.to_lowercase();

    normalized.contains("monitor")
        || normalized.contains("loopback")
        || normalized.contains("what u hear")
        || normalized.contains("stereo mix")
}

pub(crate) fn is_monitor_search_name(name: &str) -> bool {
    name == "monitor" || name == "loopback" || name == "stereo mix"
}

pub(crate) fn capture_devices() -> CaptureDevices {
    let host = cpal::default_host();

    CaptureDevices {
        microphones: microphone_devices(&host),
        system_sources: system_source_devices(&host),
    }
}

pub(crate) fn microphone_devices(host: &cpal::Host) -> Vec<CaptureDeviceInfo> {
    let mut devices = vec![CaptureDeviceInfo {
        name: "Default microphone".to_owned(),
        label: "Default microphone".to_owned(),
        default: true,
    }];

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            devices.push(capture_device_info(device.to_string(), false));
        }
    }

    dedupe_capture_devices(devices)
}

pub(crate) fn system_source_devices(host: &cpal::Host) -> Vec<CaptureDeviceInfo> {
    let mut devices = vec![CaptureDeviceInfo {
        name: "Default system output".to_owned(),
        label: "Default system output".to_owned(),
        default: true,
    }];

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            let name = device.to_string();

            if is_system_monitor_device_name(&name) {
                devices.push(capture_device_info(name, false));
            }
        }
    }

    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            devices.push(capture_device_info(device.to_string(), false));
        }
    }

    dedupe_capture_devices(devices)
}

pub(crate) fn capture_device_info(name: String, default: bool) -> CaptureDeviceInfo {
    CaptureDeviceInfo {
        label: name.clone(),
        name,
        default,
    }
}

pub(crate) fn dedupe_capture_devices(devices: Vec<CaptureDeviceInfo>) -> Vec<CaptureDeviceInfo> {
    let mut unique_devices = Vec::new();

    for device in devices {
        if unique_devices
            .iter()
            .any(|existing: &CaptureDeviceInfo| existing.name == device.name)
        {
            continue;
        }

        unique_devices.push(device);
    }

    unique_devices
}

pub(crate) fn finalize_native_source(
    source: &Option<CapturedSource>,
) -> Result<Option<FinalizedSource>, String> {
    let Some(source) = source else {
        return Ok(None);
    };
    let samples = match source.samples.lock() {
        Ok(samples) => samples.clone(),
        Err(error) => return Err(error.to_string()),
    };

    if samples.is_empty() {
        return Ok(None);
    }

    let frames = samples.len() / source.channels.max(1) as usize;

    Ok(Some(FinalizedSource {
        samples,
        sample_rate: source.sample_rate,
        channels: source.channels,
        frames,
    }))
}

pub(crate) fn append_stream_errors(
    errors: &mut Vec<CaptureError>,
    source: &Option<CapturedSource>,
) {
    let Some(source) = source else {
        return;
    };
    let Ok(stream_errors) = source.stream_errors.lock() else {
        return;
    };

    for message in stream_errors.iter() {
        errors.push(CaptureError {
            source: source.source,
            message: message.clone(),
        });
    }
}

pub(crate) fn write_mixed_recording(
    artifact_directory: &Path,
    microphone: Option<&FinalizedSource>,
    system: Option<&FinalizedSource>,
    file_name: &str,
) -> Result<(), String> {
    let Some(source) = mixed_recording_source(microphone, system) else {
        return Err("No captured audio source is available for mixed recording".to_owned());
    };

    write_pcm_wav_file(&artifact_directory.join(file_name), &source)
}

pub(crate) fn mixed_recording_source(
    microphone: Option<&FinalizedSource>,
    system: Option<&FinalizedSource>,
) -> Option<FinalizedSource> {
    let primary = microphone.or(system)?;
    let sample_rate = microphone
        .zip(system)
        .map(|(microphone, system)| microphone.sample_rate.max(system.sample_rate))
        .unwrap_or(primary.sample_rate);
    let channels = microphone
        .zip(system)
        .map(|(microphone, system)| microphone.channels.max(system.channels))
        .unwrap_or(primary.channels);
    let frames = [microphone, system]
        .into_iter()
        .flatten()
        .map(|source| converted_frame_count(source, sample_rate))
        .max()
        .unwrap_or(0);
    let mut samples = vec![0; frames * channels.max(1) as usize];

    if let Some(source) = microphone {
        mix_source_into(&mut samples, source, sample_rate, channels);
    }
    if let Some(source) = system {
        mix_source_into(&mut samples, source, sample_rate, channels);
    }

    Some(FinalizedSource {
        samples,
        sample_rate,
        channels,
        frames,
    })
}

pub(crate) fn converted_frame_count(source: &FinalizedSource, target_sample_rate: u32) -> usize {
    if source.sample_rate == target_sample_rate {
        return source.frames;
    }

    ((source.frames as u64 * target_sample_rate as u64).div_ceil(source.sample_rate as u64))
        as usize
}

pub(crate) fn mix_source_into(
    target: &mut [i16],
    source: &FinalizedSource,
    target_sample_rate: u32,
    target_channels: u16,
) {
    let target_channel_count = target_channels.max(1) as usize;
    let source_channel_count = source.channels.max(1) as usize;
    let target_frames = converted_frame_count(source, target_sample_rate);

    for target_frame in 0..target_frames {
        let source_frame = target_frame * source.sample_rate as usize / target_sample_rate as usize;

        for target_channel in 0..target_channel_count {
            let target_index = target_frame * target_channel_count + target_channel;
            let sample = source_sample(source, source_frame, target_channel, source_channel_count);

            if let Some(target_sample) = target.get_mut(target_index) {
                let mixed = *target_sample as i32 + sample as i32;
                *target_sample = mixed.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
    }
}

pub(crate) fn source_sample(
    source: &FinalizedSource,
    frame: usize,
    channel: usize,
    source_channels: usize,
) -> i16 {
    let source_channel = if source_channels == 1 {
        0
    } else {
        channel.min(source_channels - 1)
    };

    source
        .samples
        .get(frame * source_channels + source_channel)
        .copied()
        .unwrap_or_default()
}

pub(crate) fn write_pcm_wav_file(path: &Path, source: &FinalizedSource) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: source.channels,
        sample_rate: source.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|error| error.to_string())?;

    for sample in &source.samples {
        writer
            .write_sample(*sample)
            .map_err(|error| error.to_string())?;
    }

    writer.finalize().map_err(|error| error.to_string())
}

pub(crate) fn write_job_log(path: &Path, errors: &[CaptureError]) -> Result<(), String> {
    let mut lines = Vec::new();

    lines.push(serde_json::json!({
        "stage": "recording",
        "status": "complete",
        "message": "capture stopped",
    }));

    for error in errors {
        lines.push(serde_json::json!({
            "stage": "recording",
            "status": "failed",
            "source": error.source,
            "message": error.message,
        }));
    }

    let content = lines
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(path.join("job-log.jsonl"), format!("{content}\n")).map_err(|error| error.to_string())
}

pub(crate) fn write_capture_metadata(
    artifact_directory: &Path,
    backend: &str,
    sources: &[CaptureFileMetadata],
) -> Result<(), String> {
    let metadata = serde_json::json!({
        "backend": backend,
        "sources": sources,
    });

    fs::write(
        artifact_directory.join("metadata.json"),
        serde_json::to_string_pretty(&metadata).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn metadata_for_source(
    source: CaptureSource,
    path: &str,
    finalized_source: Option<&FinalizedSource>,
) -> CaptureFileMetadata {
    match finalized_source {
        Some(finalized_source) => CaptureFileMetadata {
            source,
            path: path.to_owned(),
            ready: true,
            sample_rate: Some(finalized_source.sample_rate),
            channels: Some(finalized_source.channels),
            frames: finalized_source.frames,
        },
        None => CaptureFileMetadata {
            source,
            path: path.to_owned(),
            ready: false,
            sample_rate: None,
            channels: None,
            frames: 0,
        },
    }
}

pub(crate) fn capture_errors_message(errors: &[CaptureError]) -> String {
    errors
        .iter()
        .map(|error| format!("{:?}: {}", error.source, error.message))
        .collect::<Vec<_>>()
        .join("; ")
}
