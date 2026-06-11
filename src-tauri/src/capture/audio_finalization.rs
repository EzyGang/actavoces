use std::path::Path;

use crate::artifacts::meta_directory;
use crate::capture::audio::CapturedSource;
use crate::domain::types::CaptureError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FinalizedSource {
    pub(crate) samples: Vec<i16>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) frames: usize,
}

pub(crate) fn finalize_native_source(
    source: &Option<CapturedSource>,
) -> Result<Option<FinalizedSource>, String> {
    let Some(source) = source else {
        return Ok(None);
    };
    let samples = match source.samples.lock() {
        Ok(samples) => samples.clone(),
        Err(error) => return Err(error.to_string()),
    };

    if samples.is_empty() {
        return Ok(None);
    }

    let frames = samples.len() / source.channels.max(1) as usize;

    Ok(Some(FinalizedSource {
        samples,
        sample_rate: source.sample_rate,
        channels: source.channels,
        frames,
    }))
}

pub(crate) fn append_stream_errors(
    errors: &mut Vec<CaptureError>,
    source: &Option<CapturedSource>,
) {
    let Some(source) = source else {
        return;
    };
    let Ok(stream_errors) = source.stream_errors.lock() else {
        return;
    };

    for message in stream_errors.iter() {
        errors.push(CaptureError {
            source: source.source,
            message: message.clone(),
        });
    }
}

pub(crate) fn write_mixed_recording(
    artifact_directory: &Path,
    microphone: Option<&FinalizedSource>,
    system: Option<&FinalizedSource>,
    file_name: &str,
) -> Result<(), String> {
    let Some(source) = mixed_recording_source(microphone, system) else {
        return Err("No captured audio source is available for mixed recording".to_owned());
    };

    write_pcm_wav_file(&meta_directory(artifact_directory).join(file_name), &source)
}

pub(crate) fn mixed_recording_source(
    microphone: Option<&FinalizedSource>,
    system: Option<&FinalizedSource>,
) -> Option<FinalizedSource> {
    let primary = microphone.or(system)?;
    let sample_rate = microphone
        .zip(system)
        .map(|(microphone, system)| microphone.sample_rate.max(system.sample_rate))
        .unwrap_or(primary.sample_rate);
    let channels = microphone
        .zip(system)
        .map(|(microphone, system)| microphone.channels.max(system.channels))
        .unwrap_or(primary.channels);
    let frames = [microphone, system]
        .into_iter()
        .flatten()
        .map(|source| converted_frame_count(source, sample_rate))
        .max()
        .unwrap_or(0);
    let mut samples = vec![0; frames * channels.max(1) as usize];

    if let Some(source) = microphone {
        mix_source_into(&mut samples, source, sample_rate, channels);
    }
    if let Some(source) = system {
        mix_source_into(&mut samples, source, sample_rate, channels);
    }

    Some(FinalizedSource {
        samples,
        sample_rate,
        channels,
        frames,
    })
}

pub(crate) fn converted_frame_count(source: &FinalizedSource, target_sample_rate: u32) -> usize {
    if source.sample_rate == target_sample_rate {
        return source.frames;
    }

    ((source.frames as u64 * target_sample_rate as u64).div_ceil(source.sample_rate as u64))
        as usize
}

pub(crate) fn mix_source_into(
    target: &mut [i16],
    source: &FinalizedSource,
    target_sample_rate: u32,
    target_channels: u16,
) {
    let target_channel_count = target_channels.max(1) as usize;
    let source_channel_count = source.channels.max(1) as usize;
    let target_frames = converted_frame_count(source, target_sample_rate);

    for target_frame in 0..target_frames {
        let source_frame = target_frame * source.sample_rate as usize / target_sample_rate as usize;

        for target_channel in 0..target_channel_count {
            let target_index = target_frame * target_channel_count + target_channel;
            let sample = source_sample(source, source_frame, target_channel, source_channel_count);

            if let Some(target_sample) = target.get_mut(target_index) {
                let mixed = *target_sample as i32 + sample as i32;
                *target_sample = mixed.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
    }
}

pub(crate) fn source_sample(
    source: &FinalizedSource,
    frame: usize,
    channel: usize,
    source_channels: usize,
) -> i16 {
    let source_channel = if source_channels == 1 {
        0
    } else {
        channel.min(source_channels - 1)
    };

    source
        .samples
        .get(frame * source_channels + source_channel)
        .copied()
        .unwrap_or_default()
}

pub(crate) fn write_pcm_wav_file(path: &Path, source: &FinalizedSource) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: source.channels,
        sample_rate: source.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|error| error.to_string())?;

    for sample in &source.samples {
        writer
            .write_sample(*sample)
            .map_err(|error| error.to_string())?;
    }

    writer.finalize().map_err(|error| error.to_string())
}
