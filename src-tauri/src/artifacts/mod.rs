use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::types::*;
use crate::utils::civil_datetime;
pub(crate) fn recording_stages() -> Vec<PipelineStage> {
    vec![
        stage(PipelineStageId::Recording, PipelineStageStatus::Running, 10),
        stage(
            PipelineStageId::Transcription,
            PipelineStageStatus::Pending,
            0,
        ),
        stage(
            PipelineStageId::Diarization,
            PipelineStageStatus::Pending,
            0,
        ),
        stage(PipelineStageId::Summary, PipelineStageStatus::Pending, 0),
    ]
}

pub(crate) fn stage(
    id: PipelineStageId,
    status: PipelineStageStatus,
    progress: u8,
) -> PipelineStage {
    PipelineStage {
        id,
        label: stage_label(id).to_owned(),
        status,
        progress,
        message: stage_message(id, status).to_owned(),
    }
}

pub(crate) fn stage_label(stage: PipelineStageId) -> &'static str {
    match stage {
        PipelineStageId::Recording => "Capture",
        PipelineStageId::Transcription => "Raw transcript",
        PipelineStageId::Diarization => "Diarization",
        PipelineStageId::Summary => "Summary",
    }
}

pub(crate) fn stage_message(stage: PipelineStageId, status: PipelineStageStatus) -> &'static str {
    match (stage, status) {
        (PipelineStageId::Recording, PipelineStageStatus::Complete) => "Audio capture complete",
        (PipelineStageId::Transcription, PipelineStageStatus::NeedsSetup) => {
            "Local transcription setup required"
        }
        (PipelineStageId::Summary, PipelineStageStatus::Skipped) => {
            "Summary generation is disabled"
        }
        _ => "Waiting for worker",
    }
}

pub(crate) fn capture_artifacts_with_readiness(
    path: &Path,
    mixed_ready: bool,
    microphone_ready: bool,
) -> Vec<Artifact> {
    vec![
        artifact(
            ArtifactKind::Audio,
            "Mixed WAV",
            path.join("recording.wav"),
            mixed_ready,
        ),
        artifact(
            ArtifactKind::MicrophoneAudio,
            "Microphone WAV",
            path.join("microphone.wav"),
            microphone_ready,
        ),
        artifact(
            ArtifactKind::RawTranscript,
            "Raw transcript",
            path.join("raw-transcript.md"),
            false,
        ),
        artifact(
            ArtifactKind::Segments,
            "Raw segments",
            path.join("raw-segments.json"),
            false,
        ),
        artifact(
            ArtifactKind::Diarization,
            "Diarization turns",
            path.join("diarization.json"),
            false,
        ),
        artifact(
            ArtifactKind::DiarizedTranscript,
            "Diarized transcript",
            path.join("diarized-transcript.md"),
            false,
        ),
        artifact(
            ArtifactKind::Summary,
            "Summary",
            path.join("summary.md"),
            false,
        ),
        artifact(
            ArtifactKind::Metadata,
            "Metadata",
            path.join("metadata.json"),
            true,
        ),
        artifact(
            ArtifactKind::JobLog,
            "Job log",
            path.join("job-log.jsonl"),
            true,
        ),
    ]
}

pub(crate) fn artifact(kind: ArtifactKind, label: &str, path: PathBuf, ready: bool) -> Artifact {
    Artifact {
        kind,
        label: label.to_owned(),
        path: path.display().to_string(),
        ready,
    }
}

pub(crate) fn artifact_directory(output_directory: &str, started_at: u64, title: &str) -> PathBuf {
    let date = civil_datetime(started_at);
    let slug = slugify(title);

    PathBuf::from(output_directory)
        .join(format!("{:04}", date.year))
        .join(format!("{:02}", date.month))
        .join(format!(
            "{:04}-{:02}-{:02}-{:02}{:02}{:02}-{slug}",
            date.year, date.month, date.day, date.hour, date.minute, date.second,
        ))
}

pub(crate) fn rewrite_raw_transcript_title(
    artifact_directory: &Path,
    title: &str,
) -> Result<(), String> {
    rewrite_markdown_title(
        &artifact_directory.join("raw-transcript.md"),
        "Raw transcript",
        title,
    )
}

pub(crate) fn rewrite_diarized_transcript_title(
    artifact_directory: &Path,
    title: &str,
) -> Result<(), String> {
    rewrite_markdown_title(
        &artifact_directory.join("diarized-transcript.md"),
        "Diarized transcript",
        title,
    )
}

pub(crate) fn slugify(value: &str) -> String {
    let mut slug = String::new();

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            continue;
        }

        if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    slug.trim_matches('-').to_owned()
}

fn rewrite_markdown_title(path: &Path, prefix: &str, title: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let title = title.trim();
    let heading = match title.is_empty() {
        true => format!("# {prefix}"),
        false => format!("# {prefix} - {title}"),
    };
    let rest = content
        .find('\n')
        .map(|index| &content[index + 1..])
        .unwrap_or("");

    fs::write(path, format!("{heading}\n{rest}")).map_err(|error| error.to_string())
}

#[cfg(test)]
pub(crate) fn write_test_wav_file(path: &Path, tone_hz: u32) -> Result<(), String> {
    let sample_rate = 8_000u32;
    let duration_samples = sample_rate / 5;
    let data_bytes = duration_samples * 2;
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());

    for index in 0..duration_samples {
        let phase = (index * tone_hz) % sample_rate;
        let sample = match phase < sample_rate / 2 {
            true => 2_000i16,
            false => -2_000i16,
        };
        bytes.extend_from_slice(&sample.to_le_bytes());
    }

    fs::write(path, bytes).map_err(|error| error.to_string())
}
