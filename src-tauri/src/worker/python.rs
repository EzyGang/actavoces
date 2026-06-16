use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::worker::command::{
    apply_worker_current_dir, apply_worker_path_env, resolve_uv_command, worker_command_error,
};
use crate::worker::paths::WorkerRuntimePaths;
use crate::worker::process::hide_console_window;

const WORKER_PYTHON_VERSION: &str = "3.14";
const WORKER_PYTHON_MINOR_LINK_ERROR: &str = "Failed to create Python minor version link directory";

pub(crate) fn resolve_worker_python_executable(
    paths: &WorkerRuntimePaths,
) -> Result<PathBuf, String> {
    install_worker_python(paths)?;

    if let Some(executable) = find_worker_python_executable(paths)? {
        return canonicalize_python_executable(&executable);
    }

    let mut command = Command::new(resolve_uv_command(paths));
    hide_console_window(&mut command);
    apply_worker_current_dir(&mut command, paths)?;
    apply_worker_path_env(&mut command, paths)?;
    let output = command
        .arg("python")
        .arg("find")
        .arg(WORKER_PYTHON_VERSION)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Unable to find worker Python: {error}"))?;

    if !output.status.success() {
        return Err(worker_command_error(
            "Unable to find worker Python",
            &output,
        ));
    }

    canonicalize_python_executable(Path::new(String::from_utf8_lossy(&output.stdout).trim()))
}

pub(crate) fn find_worker_python_executable(
    paths: &WorkerRuntimePaths,
) -> Result<Option<PathBuf>, String> {
    let install_directory = paths.uv_state_directory.join("python");
    let entries = match fs::read_dir(&install_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Unable to read worker Python directory {}: {error}",
                install_directory.display()
            ));
        }
    };
    let mut candidates = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| format!("Unable to read Python entry: {error}"))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Unable to inspect Python entry: {error}"))?;

        if !file_type.is_dir() || is_python_minor_version_link(&path) {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !name.starts_with(&format!("cpython-{WORKER_PYTHON_VERSION}.")) {
            continue;
        }

        if let Some(executable) = python_executable_in_directory(&path) {
            candidates.push((managed_python_patch_version(name), executable));
        }
    }

    candidates.sort_by_key(|candidate| Reverse(candidate.0));

    Ok(candidates.into_iter().map(|(_, path)| path).next())
}

pub(crate) fn resolve_worker_virtualenv_python_executable(
    paths: &WorkerRuntimePaths,
) -> Result<PathBuf, String> {
    let executable = paths
        .worker_directory
        .join(".venv")
        .join(worker_virtualenv_python_relative_path());

    if executable.exists() {
        return canonicalize_python_executable(&executable);
    }

    Err(format!(
        "Worker virtualenv Python was not found at {}",
        executable.display()
    ))
}

fn install_worker_python(paths: &WorkerRuntimePaths) -> Result<(), String> {
    let mut command = Command::new(resolve_uv_command(paths));
    hide_console_window(&mut command);
    apply_worker_current_dir(&mut command, paths)?;
    apply_worker_path_env(&mut command, paths)?;
    let output = command
        .arg("python")
        .arg("install")
        .arg(WORKER_PYTHON_VERSION)
        .arg("--no-registry")
        .arg("--no-bin")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Unable to install worker Python: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let error = worker_command_error("Unable to install worker Python", &output);
    if error.contains(WORKER_PYTHON_MINOR_LINK_ERROR)
        && find_worker_python_executable(paths)?.is_some()
    {
        return Ok(());
    }

    Err(error)
}

fn python_executable_in_directory(directory: &Path) -> Option<PathBuf> {
    let candidates = [
        directory.join(worker_python_executable_relative_path()),
        directory
            .join("install")
            .join(worker_python_executable_relative_path()),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn worker_python_executable_relative_path() -> PathBuf {
    match cfg!(windows) {
        true => PathBuf::from("python.exe"),
        false => PathBuf::from("bin").join(format!("python{WORKER_PYTHON_VERSION}")),
    }
}

fn worker_virtualenv_python_relative_path() -> PathBuf {
    match cfg!(windows) {
        true => PathBuf::from("Scripts").join("python.exe"),
        false => PathBuf::from("bin").join("python"),
    }
}

fn managed_python_patch_version(name: &str) -> u16 {
    let Some(rest) = name.strip_prefix(&format!("cpython-{WORKER_PYTHON_VERSION}.")) else {
        return 0;
    };
    let Some((patch, _)) = rest.split_once('-') else {
        return 0;
    };

    patch.parse::<u16>().unwrap_or(0)
}

#[cfg(windows)]
fn is_python_minor_version_link(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_python_minor_version_link(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn canonicalize_python_executable(executable: &Path) -> Result<PathBuf, String> {
    let parent = executable
        .parent()
        .ok_or_else(|| "Worker Python executable parent is unavailable".to_owned())?;
    let file_name = executable
        .file_name()
        .ok_or_else(|| "Worker Python executable name is unavailable".to_owned())?;
    let canonical_parent = fs::canonicalize(parent).map_err(|error| {
        format!(
            "Unable to resolve worker Python directory {}: {error}",
            parent.display()
        )
    })?;

    Ok(canonical_parent.join(file_name))
}
