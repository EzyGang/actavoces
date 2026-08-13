use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, LazyLock, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::types::WorkerEvent;
use crate::utils::unix_timestamp;
use crate::worker::paths::{resolve_worker_directory, WorkerRuntimePaths};
use crate::worker::process::hide_console_window;
use crate::worker::python::{
    resolve_worker_python_executable, resolve_worker_virtualenv_python_executable,
};

pub(crate) static WORKER_RUNTIME_PATHS: OnceLock<WorkerRuntimePaths> = OnceLock::new();
static WORKER_CLIENT: LazyLock<Mutex<WorkerClient>> =
    LazyLock::new(|| Mutex::new(WorkerClient::new()));
static WORKER_COMMAND_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) struct WorkerClient {
    process: Option<Arc<WorkerProcess>>,
}

pub(super) struct WorkerProcess {
    paths: WorkerRuntimePaths,
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    command_lock: Mutex<()>,
    events: Mutex<mpsc::Receiver<Result<WorkerEvent, String>>>,
    stderr: Arc<Mutex<String>>,
}

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
    let paths = default_worker_runtime_paths()?;
    run_worker_command_with_paths(&paths, command_name, payload)
}

pub(crate) fn run_worker_command_with_paths(
    paths: &WorkerRuntimePaths,
    command_name: &str,
    payload: serde_json::Value,
) -> Result<Vec<WorkerEvent>, String> {
    let process = {
        let mut client = WORKER_CLIENT
            .lock()
            .map_err(|error| format!("Unable to lock Python worker: {error}"))?;

        client.prepare(paths, WorkerProcess::start)?
    };
    let result = process.run(command_name, payload, None);

    if result.is_err() {
        let mut client = WORKER_CLIENT
            .lock()
            .map_err(|error| format!("Unable to lock Python worker: {error}"))?;

        client.shutdown_process(&process)?;
    }

    result
}

pub(crate) fn shutdown_worker() -> Result<(), String> {
    let mut client = WORKER_CLIENT
        .lock()
        .map_err(|error| format!("Unable to lock Python worker: {error}"))?;

    client.shutdown()
}

pub(crate) fn worker_is_running() -> bool {
    WORKER_CLIENT
        .lock()
        .is_ok_and(|mut client| client.is_running())
}

fn default_worker_runtime_paths() -> Result<WorkerRuntimePaths, String> {
    match WORKER_RUNTIME_PATHS.get() {
        Some(paths) => Ok(paths.clone()),
        None => {
            let worker_directory = resolve_worker_directory()?;

            Ok(WorkerRuntimePaths {
                uv_executable: PathBuf::from("uv"),
                uv_state_directory: worker_directory.join(".uv"),
                worker_directory,
                ffmpeg_directory: None,
            })
        }
    }
}

impl WorkerClient {
    pub(super) fn new() -> Self {
        Self { process: None }
    }

    fn prepare<F>(
        &mut self,
        paths: &WorkerRuntimePaths,
        start_process: F,
    ) -> Result<Arc<WorkerProcess>, String>
    where
        F: FnOnce(&WorkerRuntimePaths) -> Result<WorkerProcess, String>,
    {
        match self.process.as_ref() {
            Some(process) if process.is_running() && process.paths == *paths => {
                Ok(Arc::clone(process))
            }
            _ => {
                self.shutdown()?;
                let process = Arc::new(start_process(paths)?);
                self.process = Some(Arc::clone(&process));
                Ok(process)
            }
        }
    }

    #[cfg(test)]
    pub(super) fn run_with_process<F>(
        &mut self,
        paths: &WorkerRuntimePaths,
        command_name: &str,
        payload: serde_json::Value,
        start_process: F,
    ) -> Result<Vec<WorkerEvent>, String>
    where
        F: FnOnce(&WorkerRuntimePaths) -> Result<WorkerProcess, String>,
    {
        let process = self.prepare(paths, start_process)?;
        let result = process.run(command_name, payload, None);

        if result.is_err() {
            self.shutdown_process(&process)?;
        }

        result
    }

    fn shutdown_process(&mut self, process: &Arc<WorkerProcess>) -> Result<(), String> {
        if self
            .process
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, process))
        {
            self.shutdown()?;
        }

        Ok(())
    }

    pub(super) fn shutdown(&mut self) -> Result<(), String> {
        match self.process.take() {
            Some(process) => process.shutdown(),
            None => Ok(()),
        }
    }

    fn is_running(&mut self) -> bool {
        self.process
            .as_ref()
            .is_some_and(|process| process.is_running())
    }
}

impl WorkerProcess {
    fn start(paths: &WorkerRuntimePaths) -> Result<Self, String> {
        let python_executable = resolve_worker_virtualenv_python_executable(paths)?;
        let mut command = Command::new(python_executable);
        hide_console_window(&mut command);
        command
            .arg("-m")
            .arg("app.main")
            .env("PYTHONPATH", &paths.worker_directory)
            .env("PYANNOTE_METRICS_ENABLED", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        apply_worker_current_dir(&mut command, paths)?;
        apply_worker_path_env(&mut command, paths)?;

        Self::spawn(command, paths.clone())
    }

    pub(super) fn spawn(mut command: Command, paths: WorkerRuntimePaths) -> Result<Self, String> {
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
        let stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| "Worker stderr is unavailable".to_owned())?;
        let (event_sender, events) = mpsc::channel();
        let stderr = Arc::new(Mutex::new(String::new()));
        spawn_stdout_reader(stdout, event_sender);
        spawn_stderr_reader(stderr_pipe, Arc::clone(&stderr));

        Ok(Self {
            paths,
            child: Mutex::new(child),
            stdin: Mutex::new(Some(stdin)),
            events: Mutex::new(events),
            command_lock: Mutex::new(()),
            stderr,
        })
    }

    pub(super) fn run(
        &self,
        command_name: &str,
        payload: serde_json::Value,
        timeout: Option<Duration>,
    ) -> Result<Vec<WorkerEvent>, String> {
        let _command_lock = self
            .command_lock
            .lock()
            .map_err(|error| format!("Unable to lock worker command: {error}"))?;
        let command_id = format!(
            "rust-{}-{}",
            unix_timestamp(),
            WORKER_COMMAND_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let command = serde_json::json!({
            "id": &command_id,
            "name": command_name,
            "payload": payload,
        });
        let timeout = timeout.unwrap_or_else(command_timeout);
        {
            let mut stdin = self
                .stdin
                .lock()
                .map_err(|error| format!("Unable to lock worker stdin: {error}"))?;
            let stdin = stdin
                .as_mut()
                .ok_or_else(|| "Worker stdin is unavailable".to_owned())?;
            writeln!(stdin, "{command}")
                .and_then(|()| stdin.flush())
                .map_err(|error| format!("Unable to write worker command: {error}"))?;
        }

        let deadline = Instant::now() + timeout;
        let mut events = Vec::new();

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let event = self
                .events
                .lock()
                .map_err(|error| format!("Unable to lock worker events: {error}"))?
                .recv_timeout(remaining)
                .map_err(|error| match error {
                    mpsc::RecvTimeoutError::Timeout => {
                        format!(
                            "Worker command {command_name} timed out after {} seconds",
                            timeout.as_secs()
                        )
                    }
                    mpsc::RecvTimeoutError::Disconnected => {
                        self.exit_error("Worker exited before completing command")
                    }
                })??;

            if event.command_id != command_id {
                return Err(format!(
                    "Worker returned event for unexpected command {}",
                    event.command_id
                ));
            }
            let terminal = is_terminal_event(command_name, &event.event);
            events.push(event);

            if terminal {
                return Ok(events);
            }
        }
    }

    pub(super) fn shutdown(&self) -> Result<(), String> {
        self.stdin
            .lock()
            .map_err(|error| format!("Unable to lock worker stdin: {error}"))?
            .take();
        let mut child = self
            .child
            .lock()
            .map_err(|error| format!("Unable to lock Python worker: {error}"))?;
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;

        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(()),
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(error) => return Err(format!("Unable to stop Python worker: {error}")),
            }
        }

        child
            .kill()
            .map_err(|error| format!("Unable to terminate Python worker: {error}"))?;
        child
            .wait()
            .map(|_| ())
            .map_err(|error| format!("Unable to reap Python worker: {error}"))
    }

    fn is_running(&self) -> bool {
        self.child
            .lock()
            .is_ok_and(|mut child| matches!(child.try_wait(), Ok(None)))
    }

    fn exit_error(&self, context: &str) -> String {
        let status = self
            .child
            .lock()
            .ok()
            .and_then(|mut child| child.try_wait().ok().flatten())
            .map(|status| format!(" ({status})"))
            .unwrap_or_default();
        let stderr = self
            .stderr
            .lock()
            .map(|stderr| stderr.trim().to_owned())
            .unwrap_or_default();

        match stderr.is_empty() {
            true => format!("{context}{status}"),
            false => format!("{context}{status}: {stderr}"),
        }
    }
}

fn spawn_stdout_reader(
    stdout: impl Read + Send + 'static,
    sender: mpsc::Sender<Result<WorkerEvent, String>>,
) {
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let event = line
                .map_err(|error| format!("Unable to read worker output: {error}"))
                .and_then(|line| {
                    serde_json::from_str(&line)
                        .map_err(|error| format!("Unable to parse worker event: {error}"))
                });
            let failed = event.is_err();

            if sender.send(event).is_err() || failed {
                return;
            }
        }
    });
}

fn spawn_stderr_reader(mut stderr_pipe: impl Read + Send + 'static, stderr: Arc<Mutex<String>>) {
    thread::spawn(move || {
        let mut output = String::new();
        let _ = stderr_pipe.read_to_string(&mut output);
        if let Ok(mut stderr) = stderr.lock() {
            *stderr = output;
        }
    });
}

fn is_terminal_event(command_name: &str, event_name: &str) -> bool {
    if event_name == "command.failed" || event_name == "command.unsupported" {
        return true;
    }

    match command_name {
        "health.check" => event_name == "health.ok",
        "runtime.capabilities" => event_name == "runtime.capabilities",
        "models.status" => event_name == "models.status",
        "models.install" => {
            matches!(
                event_name,
                "models.install.complete" | "models.install.needs_setup"
            )
        }
        "diarization.check" => {
            matches!(event_name, "diarization.ready" | "diarization.needs_setup")
        }
        "transcribe.run" => {
            matches!(event_name, "transcribe.complete" | "transcribe.needs_setup")
        }
        "diarize.run" => {
            matches!(event_name, "diarize.complete" | "diarize.needs_setup")
        }
        "summarize.run" => {
            matches!(event_name, "summarize.complete" | "summarize.needs_setup")
        }
        _ => true,
    }
}

fn command_timeout() -> Duration {
    env::var("ACTAVOCES_WORKER_COMMAND_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_COMMAND_TIMEOUT)
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
