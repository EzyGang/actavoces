use tauri::{Emitter, Manager};

use crate::domain::types::*;
use crate::utils::lock_error;
use crate::worker::runtime::{
    bootstrap_worker, is_worker_process_running, persist_worker_setup_progress, run_worker_command,
    stop_worker_process,
};

use super::pipeline::spawn_pipeline_processing;

#[tauri::command]
pub async fn bootstrap_worker_runtime(app: tauri::AppHandle) -> Result<AppSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<ActavocesState>();

        match bootstrap_worker(&app, &state) {
            Ok(()) => {
                let repository = state.repository()?;

                let snapshot = repository.snapshot().map_err(|error| error.to_string())?;
                drop(repository);
                spawn_pipeline_processing(app.clone());

                Ok(snapshot)
            }
            Err(error) => {
                let progress = WorkerSetupProgress {
                    status: WorkerSetupStatus::Failed,
                    step: "Worker setup failed".to_owned(),
                    error: Some(error.clone()),
                };

                persist_worker_setup_progress(&state, &progress)?;
                app.emit("worker-setup-progress", progress)
                    .map_err(|emit_error| emit_error.to_string())?;

                Err(error)
            }
        }
    })
    .await
    .map_err(|error| format!("Worker setup task failed: {error}"))?
}

#[tauri::command]
pub fn get_worker_status(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
    state
        .worker_runtime
        .lock()
        .map(|runtime| {
            let mut status = runtime.status();
            status.running = is_worker_process_running();
            status
        })
        .map_err(lock_error)
}

#[tauri::command]
pub async fn start_worker(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
    let health_ok = tauri::async_runtime::spawn_blocking(|| {
        run_worker_command("health.check", serde_json::json!({}))
    })
    .await
    .map_err(|error| format!("Worker start task failed: {error}"))??
    .iter()
    .any(|event| event.event == "health.ok");
    let status = {
        let mut runtime = state.worker_runtime.lock().map_err(lock_error)?;
        runtime.running = is_worker_process_running();
        runtime.health_ok = health_ok;
        runtime.last_error = None;
        runtime.status()
    };
    persist_worker_status(&state, &status)?;

    Ok(status)
}

#[tauri::command]
pub async fn stop_worker(state: tauri::State<'_, ActavocesState>) -> Result<WorkerStatus, String> {
    tauri::async_runtime::spawn_blocking(stop_worker_process)
        .await
        .map_err(|error| format!("Worker stop task failed: {error}"))??;
    let status = {
        let mut runtime = state.worker_runtime.lock().map_err(lock_error)?;
        runtime.running = false;
        runtime.health_ok = false;
        runtime.last_error = None;
        runtime.status()
    };
    persist_worker_status(&state, &status)?;

    Ok(status)
}

#[tauri::command]
pub async fn check_worker_health(
    state: tauri::State<'_, ActavocesState>,
) -> Result<WorkerStatus, String> {
    let result = tauri::async_runtime::spawn_blocking(|| {
        run_worker_command("health.check", serde_json::json!({}))
    })
    .await
    .map_err(|error| format!("Worker health task failed: {error}"))?;
    let status = {
        let mut runtime = state.worker_runtime.lock().map_err(lock_error)?;
        runtime.running = is_worker_process_running();

        match result {
            Ok(events) if events.iter().any(|event| event.event == "health.ok") => {
                runtime.health_ok = true;
                runtime.last_error = None;
            }
            Ok(events) => {
                runtime.health_ok = false;
                runtime.last_error = Some(format!("Unexpected worker events: {}", events.len()));
            }
            Err(error) => {
                runtime.health_ok = false;
                runtime.last_error = Some(error);
            }
        }

        runtime.status()
    };
    persist_worker_status(&state, &status)?;

    Ok(status)
}

pub fn persist_worker_status(
    state: &tauri::State<'_, ActavocesState>,
    worker_status: &WorkerStatus,
) -> Result<(), String> {
    let mut repository = state.repository()?;

    repository
        .update_worker_runtime_status(worker_status)
        .map_err(|error| error.to_string())
}
