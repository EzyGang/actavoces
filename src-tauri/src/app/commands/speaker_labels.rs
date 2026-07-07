use std::fs;
use std::path::{Path, PathBuf};

use crate::artifacts::{
    clean_transcript_path, diarization_path, diarized_transcript_path, raw_segments_path,
    speaker_labeled_utterances_path, speaker_labeled_words_path,
};
use crate::diarization::{
    render_clean_transcript, render_diarized_transcript, render_speaker_labeled_utterances,
    SpeakerLabeledUtterance, SpeakerLabeledUtterancesArtifact, SpeakerLabeledWordsArtifact,
    SpeakerTurn, TranscriptSegment,
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
    let clean_transcript_path = clean_transcript_path(&artifact_directory);
    let mut diarization = read_structured_artifact::<DiarizationArtifact>(&diarization_path)?;
    let segments = read_structured_artifact::<SegmentsArtifact>(&raw_segments_path)?.segments;
    let mut changed = false;

    for turn in &mut diarization.turns {
        if turn.speaker == current {
            turn.speaker = replacement.to_owned();
            changed = true;
        }
    }
    for turn in &mut diarization.raw_turns {
        if turn.speaker == current {
            turn.speaker = replacement.to_owned();
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
        Some(utterances) => {
            fs::write(
                &transcript_path,
                render_speaker_labeled_utterances(&utterances, &recording.title),
            )
            .map_err(|error| error.to_string())?;
            fs::write(
                &clean_transcript_path,
                render_clean_transcript(
                    &segments,
                    &diarization.turns,
                    &recording.title,
                    &utterances,
                ),
            )
            .map_err(|error| error.to_string())
        }
        None => {
            fs::write(
                &transcript_path,
                render_diarized_transcript(&segments, &diarization.turns, &recording.title, &[]),
            )
            .map_err(|error| error.to_string())?;
            fs::write(
                &clean_transcript_path,
                render_clean_transcript(&segments, &diarization.turns, &recording.title, &[]),
            )
            .map_err(|error| error.to_string())
        }
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
    turns: Vec<SpeakerTurn>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    raw_turns: Vec<SpeakerTurn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    smoothing: Option<serde_json::Value>,
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
