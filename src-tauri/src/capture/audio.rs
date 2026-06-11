#[path = "audio_devices.rs"]
mod audio_devices;
#[path = "audio_finalization.rs"]
mod audio_finalization;
#[path = "audio_metadata.rs"]
mod audio_metadata;
#[path = "audio_streams.rs"]
mod audio_streams;

use std::collections::HashMap;
#[cfg(test)]
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::artifacts::{capture_artifacts_with_readiness, meta_directory, microphone_audio_path};
#[cfg(test)]
use crate::artifacts::{mixed_audio_path, write_test_wav_file};
use crate::domain::types::*;

#[allow(unused_imports)]
pub(crate) use audio_devices::capture_devices;
#[cfg(test)]
pub(crate) use audio_devices::{
    dedupe_capture_devices, is_default_system_source_name, is_system_monitor_device_name,
};
#[cfg(test)]
pub(crate) use audio_finalization::{mixed_recording_source, FinalizedSource};
pub(crate) use audio_streams::CapturedSource;

use audio_finalization::{
    append_stream_errors, finalize_native_source, write_mixed_recording, write_pcm_wav_file,
};
use audio_metadata::{
    capture_errors_message, metadata_for_source, write_capture_metadata, write_job_log,
};
use audio_streams::start_native_source;

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

#[derive(Default)]
pub(crate) struct NativeAudioCaptureBackend {
    pub(crate) active_recordings: HashMap<String, NativeCaptureSession>,
}

pub(crate) struct NativeCaptureSession {
    pub(crate) microphone: Option<CapturedSource>,
    pub(crate) system: Option<CapturedSource>,
    pub(crate) errors: Vec<CaptureError>,
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

        fs::create_dir_all(meta_directory(artifact_directory))
            .map_err(|error| error.to_string())?;

        let mut errors = session.errors;
        let microphone = finalize_native_source(&session.microphone)?;
        let system = finalize_native_source(&session.system)?;

        append_stream_errors(&mut errors, &session.microphone);
        append_stream_errors(&mut errors, &session.system);
        if let Some(microphone) = microphone.as_ref() {
            write_pcm_wav_file(&microphone_audio_path(artifact_directory), microphone)?;
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

impl fmt::Debug for NativeAudioCaptureBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeAudioCaptureBackend")
            .field("active_recordings", &self.active_recordings.keys())
            .finish()
    }
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

        fs::create_dir_all(meta_directory(artifact_directory))
            .map_err(|error| error.to_string())?;
        write_test_wav_file(&mixed_audio_path(artifact_directory), 440)?;
        write_test_wav_file(&microphone_audio_path(artifact_directory), 880)?;
        write_capture_metadata(
            artifact_directory,
            "file",
            &[
                audio_metadata::CaptureFileMetadata::ready(
                    CaptureSource::Microphone,
                    "microphone.wav",
                ),
                audio_metadata::CaptureFileMetadata::ready(CaptureSource::System, "recording.wav"),
            ],
        )?;
        fs::write(
            crate::artifacts::job_log_path(artifact_directory),
            "{\"stage\":\"recording\",\"status\":\"complete\",\"message\":\"capture stopped\"}\n",
        )
        .map_err(|error| error.to_string())?;

        Ok(CaptureResult {
            artifacts: capture_artifacts_with_readiness(artifact_directory, true, true),
            errors: Vec::new(),
        })
    }
}
