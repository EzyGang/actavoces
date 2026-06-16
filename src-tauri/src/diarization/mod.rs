mod audio;
mod render;
mod setup;
mod sortformer;
#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::artifacts::{
    diarization_path, diarized_transcript_path, meta_directory, speaker_labeled_utterances_path,
    speaker_labeled_words_path,
};

pub(crate) use render::{
    render_speaker_labeled_utterances, SpeakerLabeledUtterance, SpeakerLabeledWord,
};
pub(crate) use setup::prepare_sortformer_diarization;

const SORTFORMER_MODEL_FILE: &str = "diar_streaming_sortformer_4spk-v2.1.onnx";
const SORTFORMER_SAMPLE_RATE: u32 = 16_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SortformerDiarizationOutput {
    pub(crate) diarization_path: PathBuf,
    pub(crate) transcript_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct TranscriptSegment {
    pub(crate) start: f64,
    pub(crate) end: f64,
    pub(crate) text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct TranscriptWord {
    pub(crate) segment_id: usize,
    pub(crate) text: String,
    pub(crate) start: f64,
    pub(crate) end: f64,
    pub(crate) probability: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct SpeakerTurn {
    pub(crate) speaker: String,
    pub(crate) start: f64,
    pub(crate) end: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct DiarizationArtifact {
    turns: Vec<SpeakerTurn>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct SpeakerLabeledWordsArtifact {
    words: Vec<render::SpeakerLabeledWord>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct SpeakerLabeledUtterancesArtifact {
    utterances: Vec<render::SpeakerLabeledUtterance>,
}

pub(crate) fn run_sortformer_diarization(
    audio_path: &Path,
    output_directory: &Path,
    model_storage_directory: &Path,
    segments: &[TranscriptSegment],
    words: &[TranscriptWord],
    title: &str,
) -> Result<SortformerDiarizationOutput, String> {
    if !audio_path.exists() {
        return Err(format!(
            "Audio file does not exist: {}",
            audio_path.display()
        ));
    }

    fs::create_dir_all(meta_directory(output_directory)).map_err(|error| error.to_string())?;

    prepare_sortformer_diarization(model_storage_directory, |_| {})?;
    let model_path = model_storage_directory.join(SORTFORMER_MODEL_FILE);
    let turns = sortformer::diarize_audio(audio_path, &model_path)?;

    write_diarization_output(output_directory, segments, words, turns, title)
}

pub(crate) fn run_single_speaker_diarization(
    output_directory: &Path,
    segments: &[TranscriptSegment],
    words: &[TranscriptWord],
    title: &str,
) -> Result<SortformerDiarizationOutput, String> {
    fs::create_dir_all(meta_directory(output_directory)).map_err(|error| error.to_string())?;

    write_diarization_output(
        output_directory,
        segments,
        words,
        single_speaker_turns(segments),
        title,
    )
}

fn write_diarization_output(
    output_directory: &Path,
    segments: &[TranscriptSegment],
    words: &[TranscriptWord],
    turns: Vec<SpeakerTurn>,
    title: &str,
) -> Result<SortformerDiarizationOutput, String> {
    let diarization_path = diarization_path(output_directory);
    let transcript_path = diarized_transcript_path(output_directory);
    let labeled_words = render::speaker_labeled_words(words, &turns);
    let utterances = render::speaker_labeled_utterances(&labeled_words);

    write_json(
        &diarization_path,
        &DiarizationArtifact {
            turns: turns.clone(),
        },
    )?;
    if !labeled_words.is_empty() {
        write_json(
            &speaker_labeled_words_path(output_directory),
            &SpeakerLabeledWordsArtifact {
                words: labeled_words,
            },
        )?;
        write_json(
            &speaker_labeled_utterances_path(output_directory),
            &SpeakerLabeledUtterancesArtifact {
                utterances: utterances.clone(),
            },
        )?;
    }
    fs::write(
        &transcript_path,
        render::render_diarized_transcript(segments, &turns, title, &utterances),
    )
    .map_err(|error| error.to_string())?;

    Ok(SortformerDiarizationOutput {
        diarization_path,
        transcript_path,
    })
}

fn write_json<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    let content = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;

    fs::write(path, format!("{content}\n")).map_err(|error| error.to_string())
}

fn single_speaker_turns(segments: &[TranscriptSegment]) -> Vec<SpeakerTurn> {
    let Some(first) = segments.first() else {
        return Vec::new();
    };
    let last = segments.last().unwrap_or(first);

    vec![SpeakerTurn {
        speaker: "Speaker 1".to_owned(),
        start: first.start,
        end: last.end,
    }]
}
