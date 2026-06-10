use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::types::{Recording, SpeakerRenameInput};

pub fn rewrite_speaker_label(
    recording: &Recording,
    input: &SpeakerRenameInput,
) -> Result<(), String> {
    let current = input.speaker.trim();
    let replacement = input.replacement.trim();

    if current.is_empty() {
        return Err("Current speaker label is required".to_owned());
    }

    if replacement.is_empty() {
        return Err("Replacement speaker label is required".to_owned());
    }

    let artifact_directory = PathBuf::from(&recording.artifact_directory);
    let diarization_path = artifact_directory.join("diarization.json");
    let raw_segments_path = artifact_directory.join("raw-segments.json");
    let transcript_path = artifact_directory.join("diarized-transcript.md");
    let mut diarization = read_structured_artifact::<DiarizationArtifact>(&diarization_path)?;
    let segments = read_structured_artifact::<SegmentsArtifact>(&raw_segments_path)?.segments;
    let mut changed = false;

    for turn in &mut diarization.turns {
        if turn.speaker == current {
            turn.speaker = replacement.to_owned();
            changed = true;
        }
    }

    if !changed {
        return Err(format!("Speaker label not found: {current}"));
    }

    let content = serde_json::to_string_pretty(&diarization).map_err(|error| error.to_string())?;

    fs::write(&diarization_path, format!("{content}\n")).map_err(|error| error.to_string())?;
    fs::write(
        &transcript_path,
        render_diarized_transcript(&segments, &diarization.turns),
    )
    .map_err(|error| error.to_string())
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SegmentsArtifact {
    segments: Vec<TranscriptSegment>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DiarizationArtifact {
    turns: Vec<SpeakerTurnArtifact>,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeakerTurnArtifact {
    speaker: String,
    start: f64,
    end: f64,
}

fn read_structured_artifact<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;

    serde_json::from_str(&content)
        .map_err(|error| format!("Unable to parse {}: {error}", path.display()))
}

fn render_diarized_transcript(
    segments: &[TranscriptSegment],
    turns: &[SpeakerTurnArtifact],
) -> String {
    let mut lines = vec!["# Diarized transcript".to_owned(), String::new()];

    for turn in turns {
        let text = segment_texts_in_turn(segments, turn.start, turn.end).join(" ");

        lines.push(format!("## {}", turn.speaker));
        lines.push(String::new());
        lines.push(
            format!(
                "[{} - {}] {text}",
                format_artifact_timestamp(turn.start),
                format_artifact_timestamp(turn.end)
            )
            .trim()
            .to_owned(),
        );
        lines.push(String::new());
    }

    lines.join("\n")
}

fn segment_texts_in_turn(segments: &[TranscriptSegment], start: f64, end: f64) -> Vec<String> {
    let mut texts = Vec::new();

    for segment in segments {
        if segment.start >= start && segment.end <= end {
            texts.push(segment.text.trim().to_owned());
        }
    }

    texts
}

fn format_artifact_timestamp(value: f64) -> String {
    let total_seconds = value as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{minutes:02}:{seconds:02}")
}
