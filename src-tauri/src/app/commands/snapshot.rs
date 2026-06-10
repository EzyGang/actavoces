use crate::diagnostics;
use crate::domain::types::{ActavocesState, AppSnapshot, DiagnosticLogInput};

#[tauri::command]
pub fn get_app_snapshot(state: tauri::State<'_, ActavocesState>) -> Result<AppSnapshot, String> {
    let repository = state.repository()?;

    repository.snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn write_diagnostic_log(input: DiagnosticLogInput) -> Result<(), String> {
    diagnostics::error(&input.event, &input.message);

    Ok(())
}
