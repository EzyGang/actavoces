use tauri::{Emitter, Manager};

use crate::domain::types::*;
use crate::utils::lock_error;
use crate::worker::runtime::{
    bootstrap_worker, persist_worker_setup_progress, run_worker_command, stop_worker_process,
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
        .map(|runtime| runtime.status())
        .map_err(lock_error)
}

#[tauri::command]
pub async fn start_worker(app: tauri::AppHandle) -> Result<WorkerStatus, String> {
    update_worker_health(app).await
}

#[tauri::command]
pub async fn stop_worker(app: tauri::AppHandle) -> Result<WorkerStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        stop_worker_process()?;
        let state = app.state::<ActavocesState>();
        let status = {
            let mut runtime = state.worker_runtime.lock().map_err(lock_error)?;

            runtime.running = false;
            runtime.health_ok = false;
            runtime.status()
        };
        persist_worker_status(&state, &status)?;

        Ok(status)
    })
    .await
    .map_err(|error| format!("Worker stop task failed: {error}"))?
}

#[tauri::command]
pub async fn check_worker_health(app: tauri::AppHandle) -> Result<WorkerStatus, String> {
    update_worker_health(app).await
}

async fn update_worker_health(app: tauri::AppHandle) -> Result<WorkerStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_worker_command("health.check", serde_json::json!({}));
        let state = app.state::<ActavocesState>();
        let status = {
            let mut runtime = state.worker_runtime.lock().map_err(lock_error)?;

            runtime.running = true;
            match result {
                Ok(events) if events.iter().any(|event| event.event == "health.ok") => {
                    runtime.health_ok = true;
                    runtime.last_error = None;
                }
                Ok(events) => {
                    runtime.health_ok = false;
                    runtime.last_error =
                        Some(format!("Unexpected worker events: {}", events.len()));
                }
                Err(error) => {
                    runtime.running = false;
                    runtime.health_ok = false;
                    runtime.last_error = Some(error);
                }
            }
            runtime.status()
        };
        persist_worker_status(&state, &status)?;

        Ok(status)
    })
    .await
    .map_err(|error| format!("Worker health task failed: {error}"))?
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
