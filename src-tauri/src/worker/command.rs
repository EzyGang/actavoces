use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, LazyLock, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::types::WorkerEvent;
use crate::worker::paths::{resolve_worker_directory, WorkerRuntimePaths};
use crate::worker::process::hide_console_window;
use crate::worker::python::{
    resolve_worker_python_executable, resolve_worker_virtualenv_python_executable,
};

pub(crate) static WORKER_RUNTIME_PATHS: OnceLock<WorkerRuntimePaths> = OnceLock::new();
static WORKER_PROCESS: LazyLock<Mutex<Option<PersistentWorker>>> =
    LazyLock::new(|| Mutex::new(None));
static WORKER_COMMAND_ID: AtomicU64 = AtomicU64::new(1);
const WORKER_COMMAND_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub(crate) fn run_uv_sync(paths: &WorkerRuntimePaths) -> Result<(), String> {
    let python_executable = resolve_worker_python_executable(paths)?;
    let mut command = Command::new(resolve_uv_command(paths));
    hide_console_window(&mut command);
    apply_worker_current_dir(&mut command, paths)?;
    apply_worker_path_env(&mut command, paths)?;
    let output = command
        .arg("sync")
        .arg("--python")
        .arg(python_executable)
        .arg("--locked")
        .arg("--no-dev")
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
    let python_executable = resolve_worker_python_executable(paths)?;
    let mut command = Command::new(resolve_uv_command(paths));
    hide_console_window(&mut command);
    apply_worker_current_dir(&mut command, paths)?;
    apply_worker_path_env(&mut command, paths)?;
    let output = command
        .arg("sync")
        .arg("--python")
        .arg(python_executable)
        .arg("--locked")
        .arg("--no-dev")
        .arg("--extra")
        .arg(extra)
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
        None => {
            let worker_directory = resolve_worker_directory()?;

            WorkerRuntimePaths {
                uv_executable: PathBuf::from("uv"),
                uv_state_directory: worker_directory.join(".uv"),
                worker_directory,
                ffmpeg_directory: None,
            }
        }
    };

    run_worker_command_with_paths(&paths, command_name, payload)
}

pub(crate) fn run_worker_command_with_paths(
    paths: &WorkerRuntimePaths,
    command_name: &str,
    payload: serde_json::Value,
) -> Result<Vec<WorkerEvent>, String> {
    run_worker_command_with_timeout(paths, command_name, payload, WORKER_COMMAND_TIMEOUT)
}

pub(crate) fn shutdown_worker() -> Result<(), String> {
    let mut guard = WORKER_PROCESS.lock().map_err(|error| error.to_string())?;

    match guard.take() {
        Some(mut worker) => worker.shutdown(),
        None => Ok(()),
    }
}

pub(crate) fn run_worker_command_with_timeout(
    paths: &WorkerRuntimePaths,
    command_name: &str,
    payload: serde_json::Value,
    timeout: Duration,
) -> Result<Vec<WorkerEvent>, String> {
    let mut guard = WORKER_PROCESS.lock().map_err(|error| error.to_string())?;
    let command_id = format!("rust-{}", WORKER_COMMAND_ID.fetch_add(1, Ordering::Relaxed));
    let command = serde_json::json!({
        "id": &command_id,
        "name": command_name,
        "payload": payload,
    });
    if guard.as_ref().is_some_and(|worker| worker.paths != *paths) {
        if let Some(mut worker) = guard.take() {
            worker.shutdown()?;
        }
    }
    let worker = match guard.as_mut() {
        Some(worker) => worker,
        None => guard.insert(PersistentWorker::spawn(paths.clone())?),
    };

    match worker.run_command(&command_id, &command, timeout) {
        Ok(events) => Ok(events),
        Err(error) => {
            if let Some(mut worker) = guard.take() {
                let _ = worker.shutdown();
            }
            Err(error)
        }
    }
}

#[derive(Debug)]
struct PersistentWorker {
    paths: WorkerRuntimePaths,
    child: Child,
    stdin: ChildStdin,
    stdout: mpsc::Receiver<Result<String, String>>,
    stderr: Arc<Mutex<VecDeque<String>>>,
}

impl PersistentWorker {
    fn spawn(paths: WorkerRuntimePaths) -> Result<Self, String> {
        let python_executable = resolve_worker_virtualenv_python_executable(&paths)?;
        let mut command = Command::new(python_executable);
        hide_console_window(&mut command);
        command
            .arg("-u")
            .arg("-m")
            .arg(env::var_os("ACTAVOCES_WORKER_MODULE").unwrap_or_else(|| "app.main".into()))
            .env("PYTHONPATH", &paths.worker_directory)
            .env("PYANNOTE_METRICS_ENABLED", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        apply_worker_current_dir(&mut command, &paths)?;
        apply_worker_path_env(&mut command, &paths)?;

        let mut child = command
            .spawn()
            .map_err(|error| format!("Unable to start Python worker: {error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Worker stdin is unavailable".to_owned())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Worker stdout is unavailable".to_owned())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Worker stderr is unavailable".to_owned())?;
        let (stdout_sender, stdout_receiver) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if stdout_sender
                    .send(line.map_err(|error| format!("Unable to read worker output: {error}")))
                    .is_err()
                {
                    return;
                }
            }
        });
        let stderr_lines = Arc::new(Mutex::new(VecDeque::with_capacity(32)));
        let stderr_target = Arc::clone(&stderr_lines);
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let Ok(mut lines) = stderr_target.lock() else {
                    return;
                };
                if lines.len() == 32 {
                    lines.pop_front();
                }
                lines.push_back(line);
            }
        });

        Ok(Self {
            paths,
            child,
            stdin,
            stdout: stdout_receiver,
            stderr: stderr_lines,
        })
    }

    fn run_command(
        &mut self,
        command_id: &str,
        command: &serde_json::Value,
        timeout: Duration,
    ) -> Result<Vec<WorkerEvent>, String> {
        if let Some(status) = self
            .child
            .try_wait()
            .map_err(|error| format!("Unable to inspect Python worker: {error}"))?
        {
            return Err(self.failure(format!("Python worker exited with {status}")));
        }
        writeln!(self.stdin, "{command}")
            .and_then(|()| self.stdin.flush())
            .map_err(|error| self.failure(format!("Unable to write worker command: {error}")))?;

        let deadline = Instant::now() + timeout;
        let mut events = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(self.failure(format!(
                    "Worker command {command_id} timed out after {} seconds",
                    timeout.as_secs()
                )));
            }
            let line = self
                .stdout
                .recv_timeout(remaining)
                .map_err(|error| match error {
                    mpsc::RecvTimeoutError::Timeout => self.failure(format!(
                        "Worker command {command_id} timed out after {} seconds",
                        timeout.as_secs()
                    )),
                    mpsc::RecvTimeoutError::Disconnected => self
                        .failure("Python worker exited before completing the command".to_owned()),
                })??;
            let event: WorkerEvent = serde_json::from_str(&line).map_err(|error| {
                self.failure(format!("Malformed worker output: {error}: {line}"))
            })?;

            if event.command_id != command_id {
                return Err(self.failure(format!(
                    "Worker response command ID mismatch: expected {command_id}, received {}",
                    event.command_id
                )));
            }
            if event.event == "command.finished" {
                return Ok(events);
            }
            events.push(event);
        }
    }

    fn shutdown(&mut self) -> Result<(), String> {
        if self
            .child
            .try_wait()
            .map_err(|error| format!("Unable to inspect Python worker: {error}"))?
            .is_none()
        {
            self.child
                .kill()
                .map_err(|error| format!("Unable to stop Python worker: {error}"))?;
        }
        self.child
            .wait()
            .map_err(|error| format!("Unable to reap Python worker: {error}"))?;

        Ok(())
    }

    fn failure(&self, context: String) -> String {
        let stderr = self
            .stderr
            .lock()
            .map(|lines| lines.iter().cloned().collect::<Vec<_>>().join("\n"))
            .unwrap_or_default();

        match stderr.is_empty() {
            true => context,
            false => format!("{context}: {stderr}"),
        }
    }
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
    command
        .env("UV_CACHE_DIR", paths.uv_state_directory.join("cache"))
        .env(
            "UV_PYTHON_CACHE_DIR",
            paths.uv_state_directory.join("python-cache"),
        )
        .env(
            "UV_PYTHON_INSTALL_DIR",
            paths.uv_state_directory.join("python"),
        )
        .env(
            "UV_PYTHON_BIN_DIR",
            paths.uv_state_directory.join("python-bin"),
        )
        .env("UV_PYTHON_NO_REGISTRY", "1")
        .env("UV_PYTHON_INSTALL_REGISTRY", "0")
        .env("UV_LINK_MODE", "copy")
        .env("UV_MANAGED_PYTHON", "1")
        .env("UV_NO_CONFIG", "1")
        .env("UV_NO_SYSTEM_CONFIG", "1")
        .env(
            "UV_PROJECT_ENVIRONMENT",
            paths.worker_directory.join(".venv"),
        );

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

pub(crate) fn apply_worker_current_dir(
    command: &mut Command,
    paths: &WorkerRuntimePaths,
) -> Result<(), String> {
    fs::create_dir_all(&paths.worker_directory).map_err(|error| {
        format!(
            "Unable to create worker command directory {}: {error}",
            paths.worker_directory.display()
        )
    })?;
    command.current_dir(&paths.worker_directory);

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
