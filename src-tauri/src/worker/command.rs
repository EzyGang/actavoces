use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::domain::types::WorkerEvent;
use crate::utils::unix_timestamp;
use crate::worker::events::parse_worker_events;
use crate::worker::paths::{resolve_worker_directory, WorkerRuntimePaths};

pub(crate) static WORKER_RUNTIME_PATHS: OnceLock<WorkerRuntimePaths> = OnceLock::new();

pub(crate) fn run_uv_sync(paths: &WorkerRuntimePaths) -> Result<(), String> {
    let output = Command::new(resolve_uv_command(paths))
        .arg("sync")
        .arg("--locked")
        .arg("--no-dev")
        .current_dir(&paths.worker_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Unable to run uv sync: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    Err(worker_command_error("uv sync failed", &output))
}

pub(crate) fn run_uv_sync_extra(paths: &WorkerRuntimePaths, extra: &str) -> Result<(), String> {
    let output = Command::new(resolve_uv_command(paths))
        .arg("sync")
        .arg("--locked")
        .arg("--no-dev")
        .arg("--extra")
        .arg(extra)
        .current_dir(&paths.worker_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Unable to run uv sync --extra {extra}: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    Err(worker_command_error("uv sync diarization failed", &output))
}

pub(crate) fn run_worker_command(
    command_name: &str,
    payload: serde_json::Value,
) -> Result<Vec<WorkerEvent>, String> {
    let paths = match WORKER_RUNTIME_PATHS.get() {
        Some(paths) => paths.clone(),
        None => WorkerRuntimePaths {
            uv_executable: PathBuf::from("uv"),
            worker_directory: resolve_worker_directory()?,
            ffmpeg_directory: None,
        },
    };

    run_worker_command_with_paths(&paths, command_name, payload)
}

pub(crate) fn run_worker_command_with_paths(
    paths: &WorkerRuntimePaths,
    command_name: &str,
    payload: serde_json::Value,
) -> Result<Vec<WorkerEvent>, String> {
    let command = serde_json::json!({
        "id": format!("rust-{}", unix_timestamp()),
        "name": command_name,
        "payload": payload,
    });
    let mut command_runner = Command::new(resolve_uv_command(paths));
    command_runner
        .arg("run")
        .arg("python")
        .arg("-m")
        .arg("app.main")
        .current_dir(&paths.worker_directory)
        .env("PYTHONPATH", &paths.worker_directory)
        .env("PYANNOTE_METRICS_ENABLED", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_worker_path_env(&mut command_runner, paths)?;

    let mut process = command_runner
        .spawn()
        .map_err(|error| format!("Unable to start Python worker: {error}"))?;

    match process.stdin.as_mut() {
        Some(stdin) => {
            writeln!(stdin, "{command}")
                .map_err(|error| format!("Unable to write worker command: {error}"))?;
        }
        None => return Err("Worker stdin is unavailable".to_owned()),
    }

    let output = process
        .wait_with_output()
        .map_err(|error| format!("Unable to read worker output: {error}"))?;

    if !output.status.success() {
        return Err(worker_command_error("Worker command failed", &output));
    }

    parse_worker_events(&String::from_utf8_lossy(&output.stdout))
}

pub(crate) fn resolve_uv_command(paths: &WorkerRuntimePaths) -> PathBuf {
    match paths.uv_executable.exists() {
        true => paths.uv_executable.clone(),
        false => PathBuf::from("uv"),
    }
}

pub(crate) fn apply_worker_path_env(
    command: &mut Command,
    paths: &WorkerRuntimePaths,
) -> Result<(), String> {
    let Some(ffmpeg_directory) = &paths.ffmpeg_directory else {
        return Ok(());
    };
    let path_value = env::var_os("PATH").unwrap_or_default();
    let mut path_entries = vec![ffmpeg_directory.clone()];
    path_entries.extend(env::split_paths(&path_value));
    let joined_path = env::join_paths(path_entries)
        .map_err(|error| format!("Unable to prepare worker PATH: {error}"))?;

    command.env("PATH", joined_path);

    Ok(())
}

pub(crate) fn worker_command_error(context: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    match (stderr.is_empty(), stdout.is_empty()) {
        (false, _) => format!("{context}: {stderr}"),
        (true, false) => format!("{context}: {stdout}"),
        (true, true) => context.to_owned(),
    }
}
