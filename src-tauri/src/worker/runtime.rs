use crate::domain::types::*;
use crate::worker::command::{
    run_uv_sync, run_uv_sync_extra, run_worker_command_with_paths, WORKER_RUNTIME_PATHS,
};
use crate::worker::files::{
    prepare_uv_executable, prepare_worker_directory, prepare_worker_virtualenv,
};
use crate::worker::manifest::{worker_bootstrap_is_ready, write_worker_bootstrap_manifest};
use crate::worker::paths::worker_runtime_paths;
use crate::worker::progress::{
    emit_worker_setup_progress, refresh_runtime_capabilities_with_paths,
};
use crate::worker::source_hash::worker_source_hash;

pub(crate) use crate::worker::command::run_worker_command;
#[cfg(test)]
pub(crate) use crate::worker::command::{apply_worker_current_dir, apply_worker_path_env};
#[cfg(test)]
pub(crate) use crate::worker::events::parse_worker_events;
pub(crate) use crate::worker::events::{
    diarization_setup_message, extract_model_inventory, extract_runtime_capabilities,
    model_install_message,
};
#[cfg(test)]
pub(crate) use crate::worker::paths::worker_runtime_paths_from_local_data_directory;
pub(crate) use crate::worker::paths::WorkerRuntimePaths;
pub(crate) use crate::worker::progress::persist_worker_setup_progress;
#[cfg(test)]
pub(crate) use crate::worker::python::{
    find_worker_python_executable, resolve_worker_virtualenv_python_executable,
};
#[cfg(test)]
pub(crate) use crate::worker::source_hash::hash_worker_source_directory;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct WorkerRuntimeState {
    pub(crate) running: bool,
    pub(crate) health_ok: bool,
    pub(crate) last_error: Option<String>,
}

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
    prepare_worker_virtualenv(&paths)?;

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
    let api_key = {
        let mut repository = state.repository()?;
        repository
            .update_hugging_face_token(input.hugging_face_token.as_deref())
            .map_err(|error| error.to_string())?;
        repository
            .read_hugging_face_token()
            .map_err(|error| error.to_string())?
    };

    let Some(api_key) = api_key else {
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
