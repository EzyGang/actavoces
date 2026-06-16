use std::env;
use std::path::PathBuf;
use std::process::Command;

use tauri::Manager;

use crate::worker::process::hide_console_window;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkerRuntimePaths {
    pub(crate) uv_executable: PathBuf,
    pub(crate) worker_directory: PathBuf,
    pub(crate) uv_state_directory: PathBuf,
    pub(crate) ffmpeg_directory: Option<PathBuf>,
}

pub(crate) fn worker_runtime_paths(app: &tauri::AppHandle) -> Result<WorkerRuntimePaths, String> {
    let app_data_directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;
    let worker_directory = app_data_directory.join("worker");
    let local_app_data_directory = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| app_data_directory.clone());
    let uv_executable = local_app_data_directory
        .join("runtime")
        .join("uv")
        .join(uv_executable_name());
    let uv_state_directory = local_app_data_directory.join("uv");
    let ffmpeg_directory = resolve_ffmpeg_resource_directory(app).ok();

    Ok(WorkerRuntimePaths {
        uv_executable,
        worker_directory,
        uv_state_directory,
        ffmpeg_directory,
    })
}

pub(crate) fn uv_runtime_is_available(paths: &WorkerRuntimePaths) -> bool {
    if paths.uv_executable.exists() {
        return true;
    }

    let mut command = Command::new("uv");
    hide_console_window(&mut command);

    command
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub(crate) fn resolve_worker_resource_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resource_directory = app
        .path()
        .resource_dir()
        .map_err(|error| format!("Unable to resolve resource directory: {error}"))?;
    let bundled_worker = resource_directory.join("worker");

    if bundled_worker.join("app").join("main.py").exists() {
        return Ok(bundled_worker);
    }

    resolve_worker_directory()
}

pub(crate) fn resolve_uv_resource_executable(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resource_directory = app
        .path()
        .resource_dir()
        .map_err(|error| format!("Unable to resolve resource directory: {error}"))?;
    let candidate = resource_directory
        .join("runtime")
        .join("uv")
        .join(uv_executable_name());

    match candidate.exists() {
        true => Ok(candidate),
        false => Err("Bundled uv executable was not found".to_owned()),
    }
}

pub(crate) fn resolve_ffmpeg_resource_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resource_directory = app
        .path()
        .resource_dir()
        .map_err(|error| format!("Unable to resolve resource directory: {error}"))?;
    let ffmpeg_root = resource_directory.join("runtime").join("ffmpeg");
    let platform_directory = ffmpeg_root.join(ffmpeg_platform_name());

    if platform_directory.exists() {
        return Ok(platform_directory);
    }

    if ffmpeg_root.exists() {
        return Ok(ffmpeg_root);
    }

    Err("Bundled ffmpeg directory was not found".to_owned())
}

pub(crate) fn ffmpeg_platform_name() -> &'static str {
    match (env::consts::OS, env::consts::ARCH) {
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        _ => "unknown",
    }
}

pub(crate) fn uv_executable_name() -> &'static str {
    match cfg!(windows) {
        true => "uv.exe",
        false => "uv",
    }
}

pub(crate) fn resolve_worker_directory() -> Result<PathBuf, String> {
    let current_directory = env::current_dir()
        .map_err(|error| format!("Unable to resolve current directory: {error}"))?;
    let candidates = [
        current_directory.join("worker"),
        current_directory.join("..").join("worker"),
    ];

    for candidate in candidates {
        if candidate.join("app").join("main.py").exists() {
            return Ok(candidate);
        }
    }

    Err("Unable to find worker directory".to_owned())
}
