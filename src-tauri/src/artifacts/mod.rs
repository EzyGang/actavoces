#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::types::*;
use crate::utils::civil_datetime;

#[cfg(test)]
pub(crate) use tests::write_test_wav_file;

const META_DIRECTORY: &str = "meta";

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
            mixed_audio_path(path),
            mixed_ready,
        ),
        artifact(
            ArtifactKind::MicrophoneAudio,
            "Microphone WAV",
            microphone_audio_path(path),
            microphone_ready,
        ),
        artifact(
            ArtifactKind::RawTranscript,
            "Raw transcript",
            raw_transcript_path(path),
            false,
        ),
        artifact(
            ArtifactKind::Segments,
            "Raw segments",
            raw_segments_path(path),
            false,
        ),
        artifact(
            ArtifactKind::RawWords,
            "Raw words",
            raw_words_path(path),
            false,
        ),
        artifact(
            ArtifactKind::Diarization,
            "Diarization turns",
            diarization_path(path),
            false,
        ),
        artifact(
            ArtifactKind::DiarizedTranscript,
            "Diarized transcript",
            diarized_transcript_path(path),
            false,
        ),
        artifact(ArtifactKind::Summary, "Summary", summary_path(path), false),
        artifact(
            ArtifactKind::Metadata,
            "Metadata",
            metadata_path(path),
            true,
        ),
        artifact(ArtifactKind::JobLog, "Job log", job_log_path(path), true),
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
    let slug = title_slug(title);

    PathBuf::from(output_directory).join(format!(
        "{:04}-{:02}-{:02}-{:02}{:02}-{slug}",
        date.year, date.month, date.day, date.hour, date.minute,
    ))
}

pub(crate) fn renamed_artifact_directory(
    current_directory: &Path,
    started_at: &str,
    title: &str,
) -> PathBuf {
    let root = current_directory.parent().unwrap_or_else(|| Path::new("."));
    let started_at = started_at.parse::<u64>().unwrap_or_default();

    artifact_directory(&root.display().to_string(), started_at, title)
}

pub(crate) fn rename_artifact_directory(
    current_directory: &Path,
    target_directory: &Path,
) -> Result<PathBuf, String> {
    if current_directory == target_directory {
        return Ok(current_directory.to_path_buf());
    }

    if let Some(parent) = target_directory.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let target_directory = available_directory(target_directory);

    fs::rename(current_directory, &target_directory).map_err(|error| error.to_string())?;

    Ok(target_directory)
}

pub(crate) fn meta_directory(artifact_directory: &Path) -> PathBuf {
    artifact_directory.join(META_DIRECTORY)
}

pub(crate) fn mixed_audio_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("recording.wav")
}

pub(crate) fn microphone_audio_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("microphone.wav")
}

pub(crate) fn raw_transcript_path(artifact_directory: &Path) -> PathBuf {
    artifact_directory.join("raw-transcript.md")
}

pub(crate) fn raw_segments_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("raw-segments.json")
}

pub(crate) fn raw_words_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("raw-words.json")
}

pub(crate) fn diarization_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("diarization.json")
}

pub(crate) fn speaker_labeled_words_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("speaker-labeled-words.json")
}

pub(crate) fn speaker_labeled_utterances_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("speaker-labeled-utterances.json")
}

pub(crate) fn diarized_transcript_path(artifact_directory: &Path) -> PathBuf {
    artifact_directory.join("diarized-transcript.md")
}

pub(crate) fn summary_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("summary.md")
}

pub(crate) fn metadata_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("metadata.json")
}

pub(crate) fn job_log_path(artifact_directory: &Path) -> PathBuf {
    meta_directory(artifact_directory).join("job-log.jsonl")
}

pub(crate) fn rewrite_raw_transcript_title(
    artifact_directory: &Path,
    title: &str,
) -> Result<(), String> {
    rewrite_markdown_title(
        &raw_transcript_path(artifact_directory),
        "Raw transcript",
        title,
    )
}

pub(crate) fn rewrite_diarized_transcript_title(
    artifact_directory: &Path,
    title: &str,
) -> Result<(), String> {
    rewrite_markdown_title(
        &diarized_transcript_path(artifact_directory),
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

fn title_slug(value: &str) -> String {
    match slugify(value) {
        slug if slug.is_empty() => "untitled".to_owned(),
        slug => slug,
    }
}

fn available_directory(directory: &Path) -> PathBuf {
    if !directory.exists() {
        return directory.to_path_buf();
    }

    for suffix in 2.. {
        let candidate = suffixed_directory(directory, suffix);

        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded directory suffix search must return")
}

fn suffixed_directory(directory: &Path, suffix: u32) -> PathBuf {
    let name = directory
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("recording");

    directory.with_file_name(format!("{name}-{suffix}"))
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
