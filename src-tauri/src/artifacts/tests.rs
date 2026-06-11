use std::fs;
use std::path::Path;

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
