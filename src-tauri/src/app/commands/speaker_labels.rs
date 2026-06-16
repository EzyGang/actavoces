use std::fs;
use std::path::{Path, PathBuf};

use crate::artifacts::{
    diarization_path, diarized_transcript_path, raw_segments_path, speaker_labeled_utterances_path,
    speaker_labeled_words_path,
};
use crate::diarization::{
    render_speaker_labeled_utterances, SpeakerLabeledUtterance, SpeakerLabeledWord,
};
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
    let diarization_path = diarization_path(&artifact_directory);
    let raw_segments_path = raw_segments_path(&artifact_directory);
    let speaker_labeled_words_path = speaker_labeled_words_path(&artifact_directory);
    let speaker_labeled_utterances_path = speaker_labeled_utterances_path(&artifact_directory);
    let transcript_path = diarized_transcript_path(&artifact_directory);
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
    rewrite_speaker_labeled_words(&speaker_labeled_words_path, current, replacement)?;

    match rewrite_speaker_labeled_utterances(
        &speaker_labeled_utterances_path,
        current,
        replacement,
    )? {
        Some(utterances) => fs::write(
            &transcript_path,
            render_speaker_labeled_utterances(&utterances, &recording.title),
        )
        .map_err(|error| error.to_string()),
        None => fs::write(
            &transcript_path,
            render_diarized_transcript(&segments, &diarization.turns, &recording.title),
        )
        .map_err(|error| error.to_string()),
    }
}

fn rewrite_speaker_labeled_words(
    path: &Path,
    current: &str,
    replacement: &str,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let mut artifact = read_structured_artifact::<SpeakerLabeledWordsArtifact>(path)?;

    for word in &mut artifact.words {
        if word.speaker == current {
            word.speaker = replacement.to_owned();
        }
    }

    write_structured_artifact(path, &artifact)
}

fn rewrite_speaker_labeled_utterances(
    path: &Path,
    current: &str,
    replacement: &str,
) -> Result<Option<Vec<SpeakerLabeledUtterance>>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let mut artifact = read_structured_artifact::<SpeakerLabeledUtterancesArtifact>(path)?;

    for utterance in &mut artifact.utterances {
        if utterance.speaker == current {
            utterance.speaker = replacement.to_owned();
        }
    }

    write_structured_artifact(path, &artifact)?;

    Ok(Some(artifact.utterances))
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct SpeakerLabeledWordsArtifact {
    words: Vec<SpeakerLabeledWord>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct SpeakerLabeledUtterancesArtifact {
    utterances: Vec<SpeakerLabeledUtterance>,
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

fn write_structured_artifact<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: serde::Serialize,
{
    let content = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;

    fs::write(path, format!("{content}\n")).map_err(|error| error.to_string())
}

fn render_diarized_transcript(
    segments: &[TranscriptSegment],
    turns: &[SpeakerTurnArtifact],
    title: &str,
) -> String {
    let mut lines = vec![diarized_transcript_heading(title), String::new()];
    let mut groups: Vec<DiarizedTextGroup> = Vec::new();

    for segment in segments {
        let speaker = best_speaker_for_segment(segment, turns)
            .unwrap_or("Unknown speaker")
            .to_owned();

        match groups.last_mut() {
            Some(group) if group.speaker == speaker => {
                group.end = segment.end;
                group.texts.push(segment.text.trim().to_owned());
            }
            _ => groups.push(DiarizedTextGroup {
                speaker,
                start: segment.start,
                end: segment.end,
                texts: vec![segment.text.trim().to_owned()],
            }),
        }
    }

    for group in groups {
        lines.push(format!("## {}", group.speaker));
        lines.push(String::new());
        lines.push(
            format!(
                "[{} - {}] {}",
                format_artifact_timestamp(group.start),
                format_artifact_timestamp(group.end),
                group.texts.join(" ")
            )
            .trim()
            .to_owned(),
        );
        lines.push(String::new());
    }

    lines.join("\n")
}

fn diarized_transcript_heading(title: &str) -> String {
    match title.trim() {
        "" => "# Diarized transcript".to_owned(),
        title => format!("# Diarized transcript - {title}"),
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DiarizedTextGroup {
    speaker: String,
    start: f64,
    end: f64,
    texts: Vec<String>,
}

fn best_speaker_for_segment<'a>(
    segment: &TranscriptSegment,
    turns: &'a [SpeakerTurnArtifact],
) -> Option<&'a str> {
    let segment_midpoint = (segment.start + segment.end) / 2.0;

    turns
        .iter()
        .max_by(|left, right| {
            let left_score = speaker_turn_score(segment, segment_midpoint, left);
            let right_score = speaker_turn_score(segment, segment_midpoint, right);

            left_score.total_cmp(&right_score)
        })
        .map(|turn| turn.speaker.as_str())
}

fn speaker_turn_score(
    segment: &TranscriptSegment,
    segment_midpoint: f64,
    turn: &SpeakerTurnArtifact,
) -> f64 {
    let overlap = segment.end.min(turn.end) - segment.start.max(turn.start);

    if overlap > 0.0 {
        return overlap;
    }

    let turn_midpoint = (turn.start + turn.end) / 2.0;

    -((segment_midpoint - turn_midpoint).abs())
}

fn format_artifact_timestamp(value: f64) -> String {
    let total_seconds = value as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{minutes:02}:{seconds:02}")
}
