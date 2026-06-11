mod runtime;

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::diarization::SORTFORMER_MODEL_FILE;
use crate::domain::types::{SortformerSetupProgress, SortformerSetupStatus};

use self::runtime::ensure_onnxruntime;

const SORTFORMER_MODEL_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/diar_streaming_sortformer_4spk-v2.1.onnx";

pub(crate) fn prepare_sortformer_diarization<F>(
    model_storage_directory: &Path,
    mut report_progress: F,
) -> Result<(), String>
where
    F: FnMut(SortformerSetupProgress),
{
    report_progress(sortformer_progress(
        SortformerSetupStatus::Downloading,
        "Preparing Sortformer voice attribution",
        Some(0),
        None,
    ));

    ensure_sortformer_model(model_storage_directory, &mut report_progress)?;
    ensure_onnxruntime(model_storage_directory, &mut report_progress)?;

    report_progress(sortformer_progress(
        SortformerSetupStatus::Ready,
        "Sortformer voice attribution ready",
        Some(100),
        None,
    ));

    Ok(())
}

fn ensure_sortformer_model<F>(
    model_storage_directory: &Path,
    report_progress: &mut F,
) -> Result<PathBuf, String>
where
    F: FnMut(SortformerSetupProgress),
{
    fs::create_dir_all(model_storage_directory).map_err(|error| error.to_string())?;
    let model_path = model_storage_directory.join(SORTFORMER_MODEL_FILE);

    if model_path.exists() {
        report_progress(sortformer_progress(
            SortformerSetupStatus::Downloading,
            "Sortformer model already downloaded",
            Some(50),
            None,
        ));
        return Ok(model_path);
    }

    download_file(
        SORTFORMER_MODEL_URL,
        &model_path,
        "Sortformer diarization model",
        "Downloading Sortformer diarization model",
        0,
        50,
        report_progress,
    )?;

    Ok(model_path)
}

pub(super) fn download_file<F>(
    url: &str,
    path: &Path,
    label: &str,
    step: &str,
    start_progress: u8,
    end_progress: u8,
    report_progress: &mut F,
) -> Result<(), String>
where
    F: FnMut(SortformerSetupProgress),
{
    if path.exists() {
        report_progress(sortformer_progress(
            SortformerSetupStatus::Downloading,
            step,
            Some(end_progress),
            None,
        ));
        return Ok(());
    }

    let download_path = path.with_extension("download");
    let mut response = reqwest::blocking::get(url)
        .map_err(|error| format!("Unable to download {label}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Unable to download {label}: {error}"))?;
    let content_length = response.content_length();
    let mut output = fs::File::create(&download_path).map_err(|error| error.to_string())?;
    let mut downloaded = 0;
    let mut buffer = [0_u8; 64 * 1024];

    report_progress(sortformer_progress(
        SortformerSetupStatus::Downloading,
        step,
        Some(start_progress),
        None,
    ));

    loop {
        let bytes_read = response
            .read(&mut buffer)
            .map_err(|error| format!("Unable to download {label}: {error}"))?;

        if bytes_read == 0 {
            break;
        }

        output
            .write_all(&buffer[..bytes_read])
            .map_err(|error| error.to_string())?;
        downloaded += bytes_read as u64;

        match content_length {
            Some(total) if total > 0 => report_progress(sortformer_progress(
                SortformerSetupStatus::Downloading,
                step,
                Some(progress_between(
                    start_progress,
                    end_progress,
                    downloaded,
                    total,
                )),
                None,
            )),
            _ => (),
        }
    }

    fs::rename(&download_path, path).map_err(|error| error.to_string())
}

pub(super) fn sortformer_progress(
    status: SortformerSetupStatus,
    step: &str,
    progress: Option<u8>,
    error: Option<String>,
) -> SortformerSetupProgress {
    SortformerSetupProgress {
        status,
        step: step.to_owned(),
        progress,
        error,
    }
}

fn progress_between(start: u8, end: u8, current: u64, total: u64) -> u8 {
    let span = end.saturating_sub(start) as u64;
    let progress = start as u64 + span.saturating_mul(current).saturating_div(total);

    progress.min(end as u64) as u8
}
