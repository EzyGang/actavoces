use std::path::Path;

use crate::diarization::SORTFORMER_SAMPLE_RATE;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SortformerAudio {
    pub(super) samples: Vec<f32>,
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
}

pub(super) fn sortformer_audio(audio_path: &Path) -> Result<SortformerAudio, String> {
    let mut reader = hound::WavReader::open(audio_path).map_err(|error| error.to_string())?;
    let spec = reader.spec();
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?,
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|sample| sample.map(|sample| sample as f32 / 32768.0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?,
    };
    let mono_samples = mono_samples(&samples, spec.channels);
    let samples = resample_audio(&mono_samples, spec.sample_rate, SORTFORMER_SAMPLE_RATE);

    Ok(SortformerAudio {
        samples,
        sample_rate: SORTFORMER_SAMPLE_RATE,
        channels: 1,
    })
}

fn mono_samples(samples: &[f32], channels: u16) -> Vec<f32> {
    let channel_count = channels.max(1) as usize;

    if channel_count == 1 {
        return samples.to_vec();
    }

    samples
        .chunks(channel_count)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

fn resample_audio(samples: &[f32], source_sample_rate: u32, target_sample_rate: u32) -> Vec<f32> {
    if source_sample_rate == target_sample_rate || samples.is_empty() {
        return samples.to_vec();
    }

    let output_length = (samples.len() as u64 * target_sample_rate as u64)
        .div_ceil(source_sample_rate as u64) as usize;
    let ratio = source_sample_rate as f64 / target_sample_rate as f64;
    let mut resampled = Vec::with_capacity(output_length);

    for output_index in 0..output_length {
        let source_position = output_index as f64 * ratio;
        let source_index = source_position.floor() as usize;
        let next_index = (source_index + 1).min(samples.len() - 1);
        let fraction = (source_position - source_index as f64) as f32;
        let sample =
            samples[source_index] + (samples[next_index] - samples[source_index]) * fraction;

        resampled.push(sample);
    }

    resampled
}

#[cfg(test)]
mod tests {
    use crate::diarization::audio::{mono_samples, resample_audio};
    use crate::diarization::SORTFORMER_SAMPLE_RATE;

    #[test]
    fn mono_samples_averages_channels() {
        assert_eq!(mono_samples(&[1.0, -1.0, 0.5, 0.25], 2), vec![0.0, 0.375]);
    }

    #[test]
    fn resample_audio_converts_48khz_to_sortformer_rate() {
        let audio = vec![0.0; 48_000];
        let resampled = resample_audio(&audio, 48_000, SORTFORMER_SAMPLE_RATE);

        assert_eq!(resampled.len(), SORTFORMER_SAMPLE_RATE as usize);
    }
}
