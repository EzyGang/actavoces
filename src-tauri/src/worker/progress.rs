use tauri::Emitter;

use crate::domain::types::{ActavocesState, WorkerSetupProgress, WorkerSetupStatus};
use crate::worker::command::run_worker_command_with_paths;
use crate::worker::events::extract_runtime_capabilities;
use crate::worker::paths::WorkerRuntimePaths;

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
