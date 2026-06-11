use cpal::traits::{DeviceTrait, HostTrait};

use crate::domain::types::{CaptureDeviceInfo, CaptureDevices, CaptureSource};

pub(crate) struct CaptureDevice {
    pub(crate) device: cpal::Device,
    pub(crate) config: cpal::SupportedStreamConfig,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeviceSearchMode {
    Input,
    System,
}

pub(crate) fn capture_devices() -> CaptureDevices {
    let host = cpal::default_host();

    CaptureDevices {
        microphones: microphone_devices(&host),
        system_sources: system_source_devices(&host),
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
