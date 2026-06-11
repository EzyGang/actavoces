use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use crate::domain::types::*;
use crate::settings::{read_hugging_face_token, update_hugging_face_token};
use crate::utils::unix_timestamp;
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct WorkerRuntimeState {
    pub(crate) running: bool,
    pub(crate) health_ok: bool,
    pub(crate) last_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkerRuntimePaths {
    pub(crate) uv_executable: PathBuf,
    pub(crate) worker_directory: PathBuf,
    pub(crate) ffmpeg_directory: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkerBootstrapManifest {
    pub(crate) worker_version: String,
    pub(crate) worker_source_hash: String,
    pub(crate) uv_ready: bool,
    pub(crate) synced: bool,
    pub(crate) health_ok: bool,
    pub(crate) default_model: String,
    pub(crate) default_model_installed: bool,
}

pub(crate) static WORKER_RUNTIME_PATHS: OnceLock<WorkerRuntimePaths> = OnceLock::new();

impl WorkerRuntimeState {
    pub(crate) fn status(&self) -> WorkerStatus {
        WorkerStatus {
            running: self.running,
            health_ok: self.health_ok,
            last_error: self.last_error.clone(),
            mode: WorkerMode::CliJsonl,
        }
    }
}

pub(crate) fn bootstrap_worker(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, ActavocesState>,
) -> Result<(), String> {
    let paths = worker_runtime_paths(app)?;
    WORKER_RUNTIME_PATHS.get_or_init(|| paths.clone());

    let source_hash = worker_source_hash(app)?;

    match worker_bootstrap_is_ready(&paths, &source_hash) {
        true => {
            persist_worker_setup_progress(
                state,
                &WorkerSetupProgress {
                    status: WorkerSetupStatus::Ready,
                    step: "Worker runtime ready".to_owned(),
                    error: None,
                },
            )?;
            refresh_runtime_capabilities_with_paths(state, &paths)?;

            Ok(())
        }
        false => run_worker_bootstrap(app, state, paths, source_hash),
    }
}

pub(crate) fn run_worker_bootstrap(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, ActavocesState>,
    paths: WorkerRuntimePaths,
    source_hash: String,
) -> Result<(), String> {
    emit_worker_setup_progress(
        app,
        state,
        WorkerSetupStatus::Installing,
        "Preparing worker files",
        None,
    )?;
    prepare_worker_directory(app, &paths.worker_directory)?;
    prepare_uv_executable(app, &paths.uv_executable)?;

    emit_worker_setup_progress(
        app,
        state,
        WorkerSetupStatus::Installing,
        "Installing Python runtime",
        None,
    )?;
    run_uv_sync(&paths)?;

    emit_worker_setup_progress(
        app,
        state,
        WorkerSetupStatus::Installing,
        "Checking worker health",
        None,
    )?;
    let health_events =
        run_worker_command_with_paths(&paths, "health.check", serde_json::json!({}))?;

    if !health_events.iter().any(|event| event.event == "health.ok") {
        return Err("Worker health check did not return health.ok".to_owned());
    }
    refresh_runtime_capabilities_with_paths(state, &paths)?;

    let (settings, cuda_available) = {
        let repository = state.repository()?;

        (
            repository.settings().map_err(|error| error.to_string())?,
            repository
                .desktop_runtime_status()
                .map_err(|error| error.to_string())?
                .cuda_available,
        )
    };
    let bootstrap_compute_type = match settings.compute_type.as_str() {
        "cuda" if !cuda_available => "cpu",
        value => value,
    };

    emit_worker_setup_progress(
        app,
        state,
        WorkerSetupStatus::Installing,
        "Installing medium model",
        None,
    )?;
    let install_events = run_worker_command_with_paths(
        &paths,
        "models.install",
        serde_json::json!({
            "model": "medium",
            "computeType": bootstrap_compute_type,
            "modelStorageDirectory": settings.model_storage_directory,
        }),
    )?;

    if !install_events
        .iter()
        .any(|event| event.event == "models.install.complete")
    {
        return Err(model_install_message(&install_events));
    }

    let status_events = run_worker_command_with_paths(
        &paths,
        "models.status",
        serde_json::json!({
            "modelStorageDirectory": settings.model_storage_directory,
        }),
    )?;
    let models = extract_model_inventory(&status_events)?;
    refresh_runtime_capabilities_with_paths(state, &paths)?;

    {
        let mut repository = state.repository()?;

        repository
            .replace_model_inventory(&models)
            .map_err(|error| error.to_string())?;
        repository
            .update_diarization_runtime_ready(false)
            .map_err(|error| error.to_string())?;
    }

    write_worker_bootstrap_manifest(&paths, &source_hash)?;
    emit_worker_setup_progress(
        app,
        state,
        WorkerSetupStatus::Ready,
        "Worker runtime ready",
        None,
    )
}

pub(crate) fn run_diarization_setup(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, ActavocesState>,
    input: DiarizationSetupInput,
) -> Result<(), String> {
    bootstrap_worker(app, state)?;
    let paths = worker_runtime_paths(app)?;
    WORKER_RUNTIME_PATHS.get_or_init(|| paths.clone());
    let hugging_face_token_configured =
        update_hugging_face_token(input.hugging_face_token.as_deref())?;

    {
        let mut repository = state.repository()?;
        repository
            .update_hugging_face_token_status(hugging_face_token_configured)
            .map_err(|error| error.to_string())?;
    }

    let Some(api_key) = read_hugging_face_token()? else {
        return Err("Hugging Face token is required for speaker diarization".to_owned());
    };

    emit_worker_setup_progress(
        app,
        state,
        WorkerSetupStatus::Installing,
        "Installing speaker diarization runtime",
        None,
    )?;
    run_uv_sync_extra(&paths, "diarization")?;

    emit_worker_setup_progress(
        app,
        state,
        WorkerSetupStatus::Installing,
        "Checking pyannote speaker diarization",
        None,
    )?;
    let events = run_worker_command_with_paths(
        &paths,
        "diarization.check",
        serde_json::json!({
            "apiKey": api_key,
        }),
    )?;

    if !events
        .iter()
        .any(|event| event.event == "diarization.ready")
    {
        return Err(diarization_setup_message(&events));
    }

    {
        let mut repository = state.repository()?;
        repository
            .update_diarization_setup_skipped(false)
            .map_err(|error| error.to_string())?;
        repository
            .update_diarization_runtime_ready(true)
            .map_err(|error| error.to_string())?;
    }

    emit_worker_setup_progress(
        app,
        state,
        WorkerSetupStatus::Ready,
        "Speaker diarization runtime ready",
        None,
    )
}

pub(crate) fn refresh_runtime_capabilities_with_paths(
    state: &tauri::State<'_, ActavocesState>,
    paths: &WorkerRuntimePaths,
) -> Result<(), String> {
    let events =
        run_worker_command_with_paths(paths, "runtime.capabilities", serde_json::json!({}))?;
    let capabilities = extract_runtime_capabilities(&events)?;
    let mut repository = state.repository()?;

    repository
        .update_runtime_capabilities(&capabilities)
        .map_err(|error| error.to_string())
}

pub(crate) fn worker_runtime_paths(app: &tauri::AppHandle) -> Result<WorkerRuntimePaths, String> {
    let app_data_directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;
    let worker_directory = app_data_directory
        .join("worker")
        .join(worker_runtime_version());
    let uv_executable = app_data_directory
        .join("runtime")
        .join("uv")
        .join(uv_executable_name());
    let ffmpeg_directory = resolve_ffmpeg_resource_directory(app).ok();

    Ok(WorkerRuntimePaths {
        uv_executable,
        worker_directory,
        ffmpeg_directory,
    })
}

pub(crate) fn worker_bootstrap_is_ready(paths: &WorkerRuntimePaths, source_hash: &str) -> bool {
    if !uv_runtime_is_available(paths)
        || !paths.worker_directory.join("app").join("main.py").exists()
    {
        return false;
    }

    match read_worker_bootstrap_manifest(paths) {
        Some(manifest) => {
            manifest.worker_version == worker_runtime_version()
                && manifest.worker_source_hash == source_hash
                && manifest.uv_ready
                && manifest.synced
                && manifest.health_ok
                && manifest.default_model == "medium"
                && manifest.default_model_installed
        }
        None => false,
    }
}

pub(crate) fn uv_runtime_is_available(paths: &WorkerRuntimePaths) -> bool {
    if paths.uv_executable.exists() {
        return true;
    }

    Command::new("uv")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub(crate) fn prepare_worker_directory(
    app: &tauri::AppHandle,
    target: &Path,
) -> Result<(), String> {
    fs::create_dir_all(target)
        .map_err(|error| format!("Unable to create worker directory: {error}"))?;

    let source = resolve_worker_resource_directory(app)?;
    copy_directory(&source.join("app"), &target.join("app"))?;
    copy_file(
        &source.join("pyproject.toml"),
        &target.join("pyproject.toml"),
    )?;
    copy_file(&source.join("uv.lock"), &target.join("uv.lock"))
}

pub(crate) fn prepare_uv_executable(app: &tauri::AppHandle, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Unable to create uv runtime directory: {error}"))?;
    }

    match resolve_uv_resource_executable(app) {
        Ok(source) => copy_file(&source, target)?,
        Err(_) => {
            let status = Command::new("uv")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|error| {
                    format!("Bundled uv was not found and PATH uv is unavailable: {error}")
                })?;

            if !status.success() {
                return Err(
                    "Bundled uv was not found and PATH uv did not run successfully".to_owned(),
                );
            }

            return Ok(());
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(target)
            .map_err(|error| format!("Unable to read uv permissions: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(target, permissions)
            .map_err(|error| format!("Unable to set uv executable permissions: {error}"))?;
    }

    Ok(())
}

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

pub(crate) fn copy_directory(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        fs::remove_dir_all(target).map_err(|error| {
            format!("Unable to replace directory {}: {error}", target.display())
        })?;
    }

    fs::create_dir_all(target)
        .map_err(|error| format!("Unable to create directory {}: {error}", target.display()))?;

    for entry in fs::read_dir(source)
        .map_err(|error| format!("Unable to read directory {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read directory entry: {error}"))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory(&source_path, &target_path)?;
        } else {
            copy_file(&source_path, &target_path)?;
        }
    }

    Ok(())
}

pub(crate) fn copy_file(source: &Path, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Unable to create directory {}: {error}", parent.display()))?;
    }

    fs::copy(source, target).map_err(|error| {
        format!(
            "Unable to copy {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;

    Ok(())
}

pub(crate) fn worker_source_hash(app: &tauri::AppHandle) -> Result<String, String> {
    let source = resolve_worker_resource_directory(app)?;

    hash_worker_source_directory(&source)
}

pub(crate) fn hash_worker_source_directory(source: &Path) -> Result<String, String> {
    let mut hasher = DefaultHasher::new();

    hash_directory(&source.join("app"), source, &mut hasher)?;
    hash_file(&source.join("pyproject.toml"), source, &mut hasher)?;
    hash_file(&source.join("uv.lock"), source, &mut hasher)?;

    Ok(format!("{:016x}", hasher.finish()))
}

pub(crate) fn hash_directory(
    source: &Path,
    root: &Path,
    hasher: &mut DefaultHasher,
) -> Result<(), String> {
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("Unable to read directory {}: {error}", source.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Unable to read directory entry: {error}"))?;

    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();

        if path.is_dir() {
            hash_directory(&path, root, hasher)?;
        } else {
            hash_file(&path, root, hasher)?;
        }
    }

    Ok(())
}

pub(crate) fn hash_file(
    path: &Path,
    root: &Path,
    hasher: &mut DefaultHasher,
) -> Result<(), String> {
    let relative_path = path.strip_prefix(root).unwrap_or(path);
    let content =
        fs::read(path).map_err(|error| format!("Unable to hash {}: {error}", path.display()))?;

    relative_path.hash(hasher);
    content.hash(hasher);

    Ok(())
}

pub(crate) fn read_worker_bootstrap_manifest(
    paths: &WorkerRuntimePaths,
) -> Option<WorkerBootstrapManifest> {
    let manifest_path = paths.worker_directory.join("actavoces-worker-runtime.json");
    let content = fs::read_to_string(manifest_path).ok()?;

    serde_json::from_str(&content).ok()
}

pub(crate) fn write_worker_bootstrap_manifest(
    paths: &WorkerRuntimePaths,
    source_hash: &str,
) -> Result<(), String> {
    let manifest = WorkerBootstrapManifest {
        worker_version: worker_runtime_version(),
        worker_source_hash: source_hash.to_owned(),
        uv_ready: true,
        synced: true,
        health_ok: true,
        default_model: "medium".to_owned(),
        default_model_installed: true,
    };
    let content = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("Unable to serialize worker manifest: {error}"))?;

    fs::write(
        paths.worker_directory.join("actavoces-worker-runtime.json"),
        content,
    )
    .map_err(|error| format!("Unable to write worker manifest: {error}"))
}

pub(crate) fn emit_worker_setup_progress(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, ActavocesState>,
    status: WorkerSetupStatus,
    step: &str,
    error: Option<String>,
) -> Result<(), String> {
    let progress = WorkerSetupProgress {
        status,
        step: step.to_owned(),
        error,
    };

    persist_worker_setup_progress(state, &progress)?;
    app.emit("worker-setup-progress", progress)
        .map_err(|error| error.to_string())
}

pub(crate) fn persist_worker_setup_progress(
    state: &tauri::State<'_, ActavocesState>,
    progress: &WorkerSetupProgress,
) -> Result<(), String> {
    let mut repository = state.repository()?;

    repository
        .update_worker_setup_progress(progress)
        .map_err(|error| error.to_string())
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

pub(crate) fn worker_runtime_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

pub(crate) fn uv_executable_name() -> &'static str {
    match cfg!(windows) {
        true => "uv.exe",
        false => "uv",
    }
}

pub(crate) fn parse_worker_events(output: &str) -> Result<Vec<WorkerEvent>, String> {
    let mut events = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        events.push(
            serde_json::from_str(line)
                .map_err(|error| format!("Unable to parse worker event: {error}"))?,
        );
    }

    Ok(events)
}

pub(crate) fn extract_model_inventory(
    events: &[WorkerEvent],
) -> Result<Vec<ModelInventoryItem>, String> {
    for event in events {
        if event.event != "models.status" {
            continue;
        }

        return serde_json::from_value(
            event
                .payload
                .get("models")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
        )
        .map_err(|error| format!("Unable to parse model status: {error}"));
    }

    Err("Worker did not return model status".to_owned())
}

pub(crate) fn extract_runtime_capabilities(
    events: &[WorkerEvent],
) -> Result<RuntimeCapabilities, String> {
    for event in events {
        if event.event != "runtime.capabilities" {
            continue;
        }

        return serde_json::from_value(event.payload.clone())
            .map_err(|error| format!("Unable to parse runtime capabilities: {error}"));
    }

    Err("Worker did not return runtime capabilities".to_owned())
}

pub(crate) fn model_install_message(events: &[WorkerEvent]) -> String {
    for event in events {
        if event.event == "models.install.needs_setup" {
            let dependency = event
                .payload
                .get("dependency")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("faster-whisper");

            return format!("Model installation requires {dependency}");
        }

        if event.event == "command.failed" {
            return event
                .payload
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Model installation failed")
                .to_owned();
        }
    }

    "Model installation did not complete".to_owned()
}

pub(crate) fn diarization_setup_message(events: &[WorkerEvent]) -> String {
    for event in events {
        if event.event == "diarization.needs_setup" {
            return event
                .payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .or_else(|| {
                    event
                        .payload
                        .get("dependency")
                        .and_then(serde_json::Value::as_str)
                })
                .unwrap_or("Speaker diarization setup is incomplete")
                .to_owned();
        }

        if event.event == "command.failed" {
            return event
                .payload
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Speaker diarization setup failed")
                .to_owned();
        }
    }

    "Speaker diarization setup did not complete".to_owned()
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
