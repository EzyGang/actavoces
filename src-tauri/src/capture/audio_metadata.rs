use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::artifacts::{job_log_path, metadata_path};
use crate::capture::audio::audio_finalization::FinalizedSource;
use crate::domain::types::{CaptureError, CaptureSource};

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
    pub(crate) fn ready(source: CaptureSource, path: &str) -> Self {
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

    fs::write(job_log_path(path), format!("{content}\n")).map_err(|error| error.to_string())
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
        metadata_path(artifact_directory),
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
