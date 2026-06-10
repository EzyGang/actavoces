use crate::domain::types::*;
use crate::worker::runtime::{
    extract_model_inventory, extract_runtime_capabilities, model_install_message,
    run_worker_command,
};

#[tauri::command]
pub fn refresh_model_inventory(
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
    let settings = {
        let repository = state.repository()?;

        repository.settings().map_err(|error| error.to_string())?
    };
    let result = run_worker_command(
        "models.status",
        serde_json::json!({
            "modelStorageDirectory": settings.model_storage_directory,
        }),
    );

    match result {
        Ok(events) => {
            let models = extract_model_inventory(&events)?;
            let capabilities = run_worker_command("runtime.capabilities", serde_json::json!({}))
                .and_then(|events| extract_runtime_capabilities(&events))
                .ok();
            let mut repository = state.repository()?;

            repository
                .replace_model_inventory(&models)
                .map_err(|error| error.to_string())?;
            if let Some(capabilities) = capabilities {
                repository
                    .update_runtime_capabilities(&capabilities)
                    .map_err(|error| error.to_string())?;
            }
            repository
                .clear_worker_error()
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
        Err(error) => {
            let mut repository = state.repository()?;

            repository
                .set_worker_error(&error)
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
    }
}

#[tauri::command]
pub fn install_transcription_model(
    input: ModelInstallInput,
    state: tauri::State<'_, ActavocesState>,
) -> Result<AppSnapshot, String> {
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
    let compute_type = match settings.compute_type.as_str() {
        "cuda" if !cuda_available => "cpu",
        value => value,
    };
    let result = run_worker_command(
        "models.install",
        serde_json::json!({
            "model": input.model,
            "computeType": compute_type,
            "modelStorageDirectory": settings.model_storage_directory,
        }),
    );

    match result {
        Ok(events)
            if events
                .iter()
                .any(|event| event.event == "models.install.complete") =>
        {
            refresh_model_inventory(state)
        }
        Ok(events) => {
            let message = model_install_message(&events);
            let mut repository = state.repository()?;

            repository
                .set_worker_error(&message)
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
        Err(error) => {
            let mut repository = state.repository()?;

            repository
                .set_worker_error(&error)
                .map_err(|error| error.to_string())?;
            repository.snapshot().map_err(|error| error.to_string())
        }
    }
}
