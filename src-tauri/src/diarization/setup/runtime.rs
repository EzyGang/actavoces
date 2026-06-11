use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;

use crate::domain::types::{SortformerSetupProgress, SortformerSetupStatus};

const ORT_RUNTIME_VERSION: &str = "1.24.2";
const ORT_RUNTIME_RELEASE_URL: &str =
    "https://github.com/microsoft/onnxruntime/releases/download/v1.24.2";

static ORT_RUNTIME_READY: OnceLock<()> = OnceLock::new();

struct OrtRuntimePackage {
    archive_file: String,
    archive_format: OrtRuntimeArchiveFormat,
}

#[derive(Clone, Copy)]
enum OrtRuntimeArchiveFormat {
    Tgz,
    Zip,
}

pub(super) fn ensure_onnxruntime<F>(
    model_storage_directory: &Path,
    report_progress: &mut F,
) -> Result<(), String>
where
    F: FnMut(SortformerSetupProgress),
{
    if ORT_RUNTIME_READY.get().is_some() {
        report_progress(super::sortformer_progress(
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

fn ensure_onnxruntime_library<F>(
    model_storage_directory: &Path,
    report_progress: &mut F,
) -> Result<PathBuf, String>
where
    F: FnMut(SortformerSetupProgress),
{
    let package = ort_runtime_package()?;
    fs::create_dir_all(model_storage_directory).map_err(|error| error.to_string())?;
    let runtime_directory = model_storage_directory.join(runtime_directory_name(&package));

    if let Some(runtime_path) = find_onnxruntime_library(&runtime_directory)? {
        report_progress(super::sortformer_progress(
            SortformerSetupStatus::Downloading,
            "ONNX Runtime already downloaded",
            Some(95),
            None,
        ));
        return Ok(runtime_path);
    }

    fs::create_dir_all(&runtime_directory).map_err(|error| error.to_string())?;
    let archive_path = model_storage_directory.join(&package.archive_file);

    super::download_file(
        &ort_runtime_url(&package),
        &archive_path,
        "ONNX Runtime",
        "Downloading ONNX Runtime",
        50,
        90,
        report_progress,
    )?;
    report_progress(super::sortformer_progress(
        SortformerSetupStatus::Downloading,
        "Extracting ONNX Runtime",
        Some(92),
        None,
    ));
    extract_onnxruntime_archive(&archive_path, &runtime_directory, package.archive_format)?;

    find_onnxruntime_library(&runtime_directory)?.ok_or_else(|| {
        format!(
            "Unable to find ONNX Runtime library in {}",
            archive_path.display()
        )
    })
}

fn extract_onnxruntime_archive(
    archive_path: &Path,
    destination_directory: &Path,
    archive_format: OrtRuntimeArchiveFormat,
) -> Result<(), String> {
    match archive_format {
        OrtRuntimeArchiveFormat::Tgz => {
            extract_onnxruntime_tgz(archive_path, destination_directory)
        }
        OrtRuntimeArchiveFormat::Zip => {
            extract_onnxruntime_zip(archive_path, destination_directory)
        }
    }
}

fn extract_onnxruntime_tgz(
    archive_path: &Path,
    destination_directory: &Path,
) -> Result<(), String> {
    let archive = fs::File::open(archive_path).map_err(|error| error.to_string())?;
    let decoder = GzDecoder::new(archive);
    let mut archive = Archive::new(decoder);

    archive
        .unpack(destination_directory)
        .map_err(|error| error.to_string())
}

fn extract_onnxruntime_zip(
    archive_path: &Path,
    destination_directory: &Path,
) -> Result<(), String> {
    let archive = fs::File::open(archive_path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(archive).map_err(|error| error.to_string())?;
    let mut found_runtime = false;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        let normalized_name = file.name().replace('\\', "/");

        if !normalized_name.contains("/lib/") || !is_onnxruntime_library_name(&normalized_name) {
            continue;
        }

        let Some(file_name) = Path::new(&normalized_name).file_name() else {
            continue;
        };
        let destination_path = destination_directory.join(file_name);
        let mut output = fs::File::create(destination_path).map_err(|error| error.to_string())?;
        io::copy(&mut file, &mut output).map_err(|error| error.to_string())?;

        if is_onnxruntime_library_name(&file_name.to_string_lossy()) {
            found_runtime = true;
        }
    }

    if found_runtime {
        return Ok(());
    }

    Err(format!(
        "Unable to find ONNX Runtime library in {}",
        archive_path.display()
    ))
}

fn find_onnxruntime_library(directory: &Path) -> Result<Option<PathBuf>, String> {
    let mut libraries = Vec::new();
    find_onnxruntime_libraries(directory, &mut libraries)?;
    libraries.sort_by_key(|path| library_preference(path));

    Ok(libraries.into_iter().next())
}

fn find_onnxruntime_libraries(
    directory: &Path,
    libraries: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !directory.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| error.to_string())?;

        if file_type.is_dir() {
            find_onnxruntime_libraries(&path, libraries)?;
            continue;
        }

        if !file_type.is_file() && !file_type.is_symlink() {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };

        if is_onnxruntime_library_name(&file_name.to_string_lossy()) {
            libraries.push(path);
        }
    }

    Ok(())
}

fn is_onnxruntime_library_name(file_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        file_name == "onnxruntime.dll"
    }

    #[cfg(target_os = "macos")]
    {
        file_name == "libonnxruntime.dylib"
            || file_name.starts_with("libonnxruntime.") && file_name.ends_with(".dylib")
    }

    #[cfg(target_os = "linux")]
    {
        file_name == "libonnxruntime.so" || file_name.starts_with("libonnxruntime.so.")
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

fn library_preference(path: &Path) -> u8 {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return 2;
    };

    match file_name {
        "onnxruntime.dll" | "libonnxruntime.dylib" | "libonnxruntime.so" => 0,
        _ => 1,
    }
}

fn ort_runtime_package() -> Result<OrtRuntimePackage, String> {
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "aarch64") => "linux-aarch64",
        ("linux", "x86_64") => "linux-x64",
        ("macos", "aarch64") => "osx-arm64",
        ("windows", "aarch64") => "win-arm64",
        ("windows", "x86_64") => "win-x64",
        (os, arch) => {
            return Err(format!(
                "ONNX Runtime automatic download is not configured for {os}-{arch}"
            ));
        }
    };
    let extension = match std::env::consts::OS {
        "windows" => "zip",
        _ => "tgz",
    };
    let archive_format = match std::env::consts::OS {
        "windows" => OrtRuntimeArchiveFormat::Zip,
        _ => OrtRuntimeArchiveFormat::Tgz,
    };
    let archive_file = format!("onnxruntime-{platform}-{ORT_RUNTIME_VERSION}.{extension}");

    Ok(OrtRuntimePackage {
        archive_file,
        archive_format,
    })
}

fn runtime_directory_name(package: &OrtRuntimePackage) -> &str {
    package
        .archive_file
        .as_str()
        .trim_end_matches(".zip")
        .trim_end_matches(".tgz")
}

fn ort_runtime_url(package: &OrtRuntimePackage) -> String {
    format!("{ORT_RUNTIME_RELEASE_URL}/{}", package.archive_file)
}
