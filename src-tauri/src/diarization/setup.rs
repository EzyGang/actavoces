use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use zip::ZipArchive;

use crate::diarization::SORTFORMER_MODEL_FILE;
use crate::domain::types::{SortformerSetupProgress, SortformerSetupStatus};

const SORTFORMER_MODEL_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/diar_streaming_sortformer_4spk-v2.1.onnx";
const ORT_RUNTIME_DIRECTORY: &str = "onnxruntime-1.24.2";
const ORT_RUNTIME_ARCHIVE_FILE: &str = "onnxruntime-win-x64-1.24.2.zip";
const ORT_RUNTIME_URL: &str =
    "https://github.com/microsoft/onnxruntime/releases/download/v1.24.2/onnxruntime-win-x64-1.24.2.zip";

static ORT_RUNTIME_READY: OnceLock<()> = OnceLock::new();

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

fn ensure_onnxruntime<F>(
    model_storage_directory: &Path,
    report_progress: &mut F,
) -> Result<(), String>
where
    F: FnMut(SortformerSetupProgress),
{
    if ORT_RUNTIME_READY.get().is_some() {
        report_progress(sortformer_progress(
            SortformerSetupStatus::Downloading,
            "ONNX Runtime already loaded",
            Some(95),
            None,
        ));
        return Ok(());
    }

    let runtime_path = ensure_onnxruntime_library(model_storage_directory, report_progress)?;

    ort::init_from(&runtime_path)
        .map_err(|error| format!("Unable to load ONNX Runtime: {error}"))?
        .commit();
    let _ = ORT_RUNTIME_READY.set(());

    Ok(())
}

#[cfg(target_os = "windows")]
fn ensure_onnxruntime_library<F>(
    model_storage_directory: &Path,
    report_progress: &mut F,
) -> Result<PathBuf, String>
where
    F: FnMut(SortformerSetupProgress),
{
    fs::create_dir_all(model_storage_directory).map_err(|error| error.to_string())?;
    let runtime_directory = model_storage_directory.join(ORT_RUNTIME_DIRECTORY);
    let runtime_path = runtime_directory.join("onnxruntime.dll");

    if runtime_path.exists() {
        report_progress(sortformer_progress(
            SortformerSetupStatus::Downloading,
            "ONNX Runtime already downloaded",
            Some(95),
            None,
        ));
        return Ok(runtime_path);
    }

    fs::create_dir_all(&runtime_directory).map_err(|error| error.to_string())?;
    let archive_path = model_storage_directory.join(ORT_RUNTIME_ARCHIVE_FILE);

    download_file(
        ORT_RUNTIME_URL,
        &archive_path,
        "ONNX Runtime",
        "Downloading ONNX Runtime",
        50,
        90,
        report_progress,
    )?;
    report_progress(sortformer_progress(
        SortformerSetupStatus::Downloading,
        "Extracting ONNX Runtime",
        Some(92),
        None,
    ));
    extract_onnxruntime_dlls(&archive_path, &runtime_directory)?;

    Ok(runtime_path)
}

#[cfg(not(target_os = "windows"))]
fn ensure_onnxruntime_library<F>(
    _model_storage_directory: &Path,
    _report_progress: &mut F,
) -> Result<PathBuf, String>
where
    F: FnMut(SortformerSetupProgress),
{
    Err(
        "Sortformer diarization currently downloads ONNX Runtime automatically only on Windows"
            .to_owned(),
    )
}

fn download_file<F>(
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

fn extract_onnxruntime_dlls(
    archive_path: &Path,
    destination_directory: &Path,
) -> Result<(), String> {
    let archive = fs::File::open(archive_path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(archive).map_err(|error| error.to_string())?;
    let mut found_runtime = false;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        let normalized_name = file.name().replace('\\', "/");

        if !normalized_name.contains("/lib/") || !normalized_name.ends_with(".dll") {
            continue;
        }

        let Some(file_name) = Path::new(&normalized_name).file_name() else {
            continue;
        };
        let destination_path = destination_directory.join(file_name);
        let mut output = fs::File::create(destination_path).map_err(|error| error.to_string())?;
        io::copy(&mut file, &mut output).map_err(|error| error.to_string())?;

        if file_name.to_string_lossy() == "onnxruntime.dll" {
            found_runtime = true;
        }
    }

    if found_runtime {
        return Ok(());
    }

    Err(format!(
        "Unable to find onnxruntime.dll in {}",
        archive_path.display()
    ))
}

fn sortformer_progress(
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
