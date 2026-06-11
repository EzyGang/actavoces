use std::path::Path;

use parakeet_rs::sortformer::{DiarizationConfig, Sortformer};

use crate::diarization::audio::sortformer_audio;
use crate::diarization::{SpeakerTurn, SORTFORMER_SAMPLE_RATE};

pub(super) fn diarize_audio(
    audio_path: &Path,
    model_path: &Path,
) -> Result<Vec<SpeakerTurn>, String> {
    let audio = sortformer_audio(audio_path)?;

    let mut sortformer = Sortformer::with_config(
        model_path
            .to_str()
            .ok_or_else(|| "Sortformer model path is not valid UTF-8".to_owned())?,
        None,
        DiarizationConfig::callhome(),
    )
    .map_err(|error| error.to_string())?;
    let diarized_segments = sortformer
        .diarize(audio.samples, audio.sample_rate, audio.channels)
        .map_err(|error| error.to_string())?;

    Ok(diarized_segments
        .iter()
        .map(|segment| SpeakerTurn {
            speaker: format!("Speaker {}", segment.speaker_id + 1),
            start: segment.start as f64 / SORTFORMER_SAMPLE_RATE as f64,
            end: segment.end as f64 / SORTFORMER_SAMPLE_RATE as f64,
        })
        .collect())
}
