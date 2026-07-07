mod audio;
mod clean;
mod render;
mod setup;
mod smoothing;
mod sortformer;
#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::artifacts::{
    clean_transcript_path, diarization_path, diarized_transcript_path, meta_directory,
    speaker_labeled_utterances_path, speaker_labeled_words_path,
};

pub(crate) use clean::render_clean_transcript;
pub(crate) use render::{
    render_diarized_transcript, render_speaker_labeled_utterances, SpeakerLabeledUtterance,
    SpeakerLabeledUtterancesArtifact, SpeakerLabeledWordsArtifact,
};
pub(crate) use setup::prepare_sortformer_diarization;

const SORTFORMER_MODEL_FILE: &str = "diar_streaming_sortformer_4spk-v2.1.onnx";
const SORTFORMER_SAMPLE_RATE: u32 = 16_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SortformerDiarizationOutput {
    pub(crate) diarization_path: PathBuf,
    pub(crate) transcript_path: PathBuf,
    pub(crate) clean_transcript_path: PathBuf,
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
#[serde(rename_all = "camelCase")]
struct DiarizationArtifact {
    turns: Vec<SpeakerTurn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_turns: Option<Vec<SpeakerTurn>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    smoothing: Option<DiarizationSmoothingMetadata>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiarizationSmoothingMetadata {
    policy: &'static str,
    tiny_gap_seconds: f64,
    short_island_seconds: f64,
    backchannel_preserve_seconds: f64,
    rapid_flip_window_seconds: f64,
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
    let raw_turns = sortformer::diarize_audio(audio_path, &model_path)?;
    let turns = smoothing::smooth_turns(&raw_turns);

    write_diarization_output(
        output_directory,
        segments,
        words,
        turns,
        Some(raw_turns),
        title,
    )
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
        None,
        title,
    )
}

fn write_diarization_output(
    output_directory: &Path,
    segments: &[TranscriptSegment],
    words: &[TranscriptWord],
    turns: Vec<SpeakerTurn>,
    raw_turns: Option<Vec<SpeakerTurn>>,
    title: &str,
) -> Result<SortformerDiarizationOutput, String> {
    let diarization_path = diarization_path(output_directory);
    let transcript_path = diarized_transcript_path(output_directory);
    let labeled_words = render::speaker_labeled_words(words, &turns);
    let utterances = render::speaker_labeled_utterances(&labeled_words);
    let smoothing = raw_turns.as_ref().map(|_| DiarizationSmoothingMetadata {
        policy: smoothing::SMOOTHING_POLICY,
        tiny_gap_seconds: smoothing::TINY_GAP_SECONDS,
        short_island_seconds: smoothing::SHORT_ISLAND_SECONDS,
        backchannel_preserve_seconds: smoothing::BACKCHANNEL_PRESERVE_SECONDS,
        rapid_flip_window_seconds: smoothing::RAPID_FLIP_WINDOW_SECONDS,
    });

    write_json(
        &diarization_path,
        &DiarizationArtifact {
            turns: turns.clone(),
            raw_turns,
            smoothing,
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
    let clean_transcript_path = clean_transcript_path(output_directory);
    fs::write(
        &clean_transcript_path,
        clean::render_clean_transcript(segments, &turns, title, &utterances),
    )
    .map_err(|error| error.to_string())?;

    Ok(SortformerDiarizationOutput {
        diarization_path,
        transcript_path,
        clean_transcript_path,
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
