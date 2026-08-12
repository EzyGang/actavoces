use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::capture::audio::{
    finalize_native_source, start_native_source, write_pcm_wav_file, CapturedSource,
};
use crate::domain::types::{ActavocesState, AppSettings, CaptureSource, DictationShortcutMode};
use crate::utils::lock_error;
use crate::worker::runtime::run_worker_command;

pub(crate) const DICTATION_STATE_EVENT: &str = "dictation-state-update";
pub(crate) const MAX_DICTATION_DURATION: Duration = Duration::from_secs(15 * 60);
const DICTATION_TEMP_DIRECTORY: &str = "dictation-runtime";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DictationState {
    Idle,
    Capturing,
    Finalizing,
    Transcribing,
    Copied,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DictationShortcutEvent {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DictationAction {
    Start,
    Stop,
    Ignore,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DictationStateUpdate {
    pub(crate) session_id: Option<String>,
    pub(crate) state: DictationState,
    pub(crate) error: Option<String>,
    pub(crate) text: Option<String>,
}

pub(crate) struct DictationRuntime {
    update: DictationStateUpdate,
    shortcut_down: bool,
    starting: bool,
    started_at: Option<SystemTime>,
    temporary_directory: Option<PathBuf>,
    microphone: Option<CapturedSource>,
}

impl fmt::Debug for DictationRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DictationRuntime")
            .field("update", &self.update)
            .field("shortcut_down", &self.shortcut_down)
            .field("starting", &self.starting)
            .field("started_at", &self.started_at)
            .field("temporary_directory", &self.temporary_directory)
            .field("microphone_active", &self.microphone.is_some())
            .finish()
    }
}

impl Default for DictationRuntime {
    fn default() -> Self {
        Self {
            update: DictationStateUpdate {
                session_id: None,
                state: DictationState::Idle,
                error: None,
                text: None,
            },
            shortcut_down: false,
            starting: false,
            started_at: None,
            temporary_directory: None,
            microphone: None,
        }
    }
}

impl DictationRuntime {
    #[must_use]
    pub(crate) fn update(&self) -> DictationStateUpdate {
        self.update.clone()
    }

    #[must_use]
    pub(crate) fn blocks_recording(&self) -> bool {
        self.starting
            || matches!(
                self.update.state,
                DictationState::Capturing
                    | DictationState::Finalizing
                    | DictationState::Transcribing
            )
    }
    #[cfg(test)]
    pub(crate) fn set_state_for_test(&mut self, state: DictationState) {
        self.transition(state, None, None);
    }

    #[cfg(test)]
    pub(crate) fn set_started_at_for_test(&mut self, started_at: SystemTime) {
        self.started_at = Some(started_at);
    }

    pub(crate) fn shortcut_action(
        &mut self,
        mode: DictationShortcutMode,
        event: DictationShortcutEvent,
    ) -> DictationAction {
        match event {
            DictationShortcutEvent::Pressed if self.shortcut_down => DictationAction::Ignore,
            DictationShortcutEvent::Pressed => {
                self.shortcut_down = true;
                match (mode, self.update.state) {
                    (
                        _,
                        DictationState::Idle
                        | DictationState::Copied
                        | DictationState::Cancelled
                        | DictationState::Failed,
                    ) => DictationAction::Start,
                    (DictationShortcutMode::Toggle, DictationState::Capturing) => {
                        DictationAction::Stop
                    }
                    _ => DictationAction::Ignore,
                }
            }
            DictationShortcutEvent::Released if !self.shortcut_down => DictationAction::Ignore,
            DictationShortcutEvent::Released => {
                self.shortcut_down = false;
                match (mode, self.update.state) {
                    (DictationShortcutMode::PushToTalk, DictationState::Capturing) => {
                        DictationAction::Stop
                    }
                    _ => DictationAction::Ignore,
                }
            }
        }
    }

    #[must_use]
    pub(crate) fn duration_limit_reached(&self, now: SystemTime) -> bool {
        self.update.state == DictationState::Capturing
            && self
                .started_at
                .and_then(|started_at| now.duration_since(started_at).ok())
                .is_some_and(|duration| duration >= MAX_DICTATION_DURATION)
    }

    fn transition(&mut self, state: DictationState, error: Option<String>, text: Option<String>) {
        self.update.state = state;
        self.update.error = error;
        self.update.text = text;
    }

    fn reset_session(&mut self) {
        self.microphone = None;
        self.starting = false;
        self.started_at = None;
        self.temporary_directory = None;
    }
}

pub(crate) fn cleanup_stale_dictations(app_data_directory: &Path) -> Result<(), String> {
    remove_directory(&app_data_directory.join(DICTATION_TEMP_DIRECTORY))
}

pub(crate) fn dispatch_shortcut(app: tauri::AppHandle, event: DictationShortcutEvent) {
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = handle_shortcut(&app, event) {
            fail_active_dictation(&app, error);
        }
    });
}

#[tauri::command]
pub(crate) async fn cancel_active_dictation(
    app: tauri::AppHandle,
) -> Result<DictationStateUpdate, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<ActavocesState>();
        let mut runtime = state.dictation_runtime.lock().map_err(lock_error)?;

        if runtime.update.state != DictationState::Capturing {
            return Err("No dictation capture is active".to_owned());
        }

        cleanup_runtime(&mut runtime)?;
        runtime.transition(DictationState::Cancelled, None, None);
        emit_update(&app, &runtime.update);

        Ok(runtime.update())
    })
    .await
    .map_err(|error| format!("Dictation cancellation task failed: {error}"))?
}

#[tauri::command]
pub(crate) fn get_dictation_status(
    state: tauri::State<'_, ActavocesState>,
) -> Result<DictationStateUpdate, String> {
    dictation_status(&state)
}

pub(crate) fn dictation_status(state: &ActavocesState) -> Result<DictationStateUpdate, String> {
    state
        .dictation_runtime
        .lock()
        .map_err(lock_error)
        .map(|runtime| runtime.update())
}

fn handle_shortcut(app: &tauri::AppHandle, event: DictationShortcutEvent) -> Result<(), String> {
    let state = app.state::<ActavocesState>();
    let settings = {
        let repository = state.repository()?;
        repository.settings().map_err(|error| error.to_string())?
    };
    let action = {
        let mut runtime = state.dictation_runtime.lock().map_err(lock_error)?;
        runtime.shortcut_action(settings.dictation_shortcut_mode, event)
    };

    match action {
        DictationAction::Start => start_dictation(app, &settings),
        DictationAction::Stop => stop_and_transcribe(app, &settings),
        DictationAction::Ignore => Ok(()),
    }
}

fn start_dictation(app: &tauri::AppHandle, settings: &AppSettings) -> Result<(), String> {
    let state = app.state::<ActavocesState>();
    let mut runtime = state.dictation_runtime.lock().map_err(lock_error)?;
    if runtime.blocks_recording() {
        return Err("A dictation is already being processed".to_owned());
    }
    runtime.starting = true;
    let recording_active = {
        let repository = state.repository()?;
        repository
            .active_recording()
            .map_err(|error| error.to_string())?
            .is_some()
    };
    if recording_active {
        runtime.starting = false;
        return Err("Dictation cannot start while meeting capture is active".to_owned());
    }

    let session_id = new_session_id();
    let app_data_directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;
    let temporary_directory = app_data_directory
        .join(DICTATION_TEMP_DIRECTORY)
        .join(&session_id);
    fs::create_dir_all(&temporary_directory)
        .map_err(|error| format!("Unable to create dictation temporary storage: {error}"))?;
    runtime.temporary_directory = Some(temporary_directory.clone());
    let microphone = start_native_source(
        &cpal::default_host(),
        CaptureSource::Microphone,
        &settings.microphone_device,
    )?;

    runtime.starting = false;
    runtime.update.session_id = Some(session_id.clone());

    runtime.started_at = Some(SystemTime::now());
    runtime.microphone = Some(microphone);
    runtime.transition(DictationState::Capturing, None, None);
    emit_update(app, &runtime.update);
    drop(runtime);
    spawn_duration_limit(app.clone(), session_id);

    Ok(())
}

fn stop_and_transcribe(app: &tauri::AppHandle, settings: &AppSettings) -> Result<(), String> {
    let state = app.state::<ActavocesState>();
    let (session_id, temporary_directory, audio_path) = {
        let mut runtime = state.dictation_runtime.lock().map_err(lock_error)?;
        if runtime.update.state != DictationState::Capturing {
            return Err("No dictation capture is active".to_owned());
        }
        runtime.transition(DictationState::Finalizing, None, None);
        emit_update(app, &runtime.update);
        let temporary_directory = runtime
            .temporary_directory
            .clone()
            .ok_or_else(|| "Dictation temporary storage is unavailable".to_owned())?;
        let audio_path = temporary_directory.join("dictation.wav");
        let microphone = finalize_native_source(&runtime.microphone)?
            .ok_or_else(|| "Dictation captured no microphone audio".to_owned())?;
        runtime.microphone = None;
        write_pcm_wav_file(&audio_path, &microphone)?;
        runtime.transition(DictationState::Transcribing, None, None);
        emit_update(app, &runtime.update);
        (
            runtime.update.session_id.clone().unwrap_or_default(),
            temporary_directory,
            audio_path,
        )
    };

    let events = run_worker_command(
        "transcribe.run",
        dictation_transcription_payload(settings, &audio_path, &temporary_directory),
    )?;
    let transcript_path = transcript_path(&events)?;
    let text = fs::read_to_string(&transcript_path)
        .map_err(|error| format!("Unable to read dictation transcript: {error}"))?;
    let text = transcript_body(&text);
    copy_to_clipboard(&text)?;
    remove_directory(&temporary_directory)?;

    let mut runtime = state.dictation_runtime.lock().map_err(lock_error)?;
    if runtime.update.session_id.as_deref() != Some(&session_id) {
        return Ok(());
    }
    runtime.reset_session();
    runtime.transition(DictationState::Copied, None, Some(text));
    emit_update(app, &runtime.update);

    Ok(())
}

fn fail_active_dictation(app: &tauri::AppHandle, error: String) {
    let state = app.state::<ActavocesState>();
    let Ok(mut runtime) = state.dictation_runtime.lock() else {
        let _ = app.emit("app-error", error);
        return;
    };
    let _ = cleanup_runtime(&mut runtime);
    runtime.transition(DictationState::Failed, Some(error.clone()), None);
    emit_update(app, &runtime.update);
    let _ = app.emit("app-error", error);
}

fn spawn_duration_limit(app: tauri::AppHandle, session_id: String) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(MAX_DICTATION_DURATION).await;
        tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<ActavocesState>();
            let reached = state
                .dictation_runtime
                .lock()
                .map(|runtime| {
                    runtime.update.session_id.as_deref() == Some(&session_id)
                        && runtime.duration_limit_reached(SystemTime::now())
                })
                .unwrap_or(false);
            if reached {
                fail_active_dictation(&app, "Maximum dictation duration reached".to_owned());
            }
        });
    });
}

fn cleanup_runtime(runtime: &mut DictationRuntime) -> Result<(), String> {
    runtime.microphone = None;
    let result = match runtime.temporary_directory.as_ref() {
        Some(path) => remove_directory(path),
        None => Ok(()),
    };
    runtime.reset_session();
    result
}

fn remove_directory(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Unable to remove {}: {error}", path.display())),
    }
}

fn dictation_transcription_payload(
    settings: &AppSettings,
    audio_path: &Path,
    output_directory: &Path,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "audioPath": audio_path,
        "outputDirectory": output_directory,
        "title": "Dictation",
        "model": settings.dictation_whisper_model,
        "language": settings.dictation_language,
        "computeType": settings.compute_type,
        "modelStorageDirectory": settings.model_storage_directory,
        "transcriptionProfile": "dictation",
    });
    let context = settings.dictation_context.trim();
    if !context.is_empty() {
        payload["transcriptionContext"] = serde_json::json!(context);
    }
    payload
}

fn transcript_path(events: &[crate::domain::types::WorkerEvent]) -> Result<PathBuf, String> {
    for event in events {
        if event.event == "command.failed" {
            return Err(event
                .payload
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Dictation transcription failed")
                .to_owned());
        }
        if event.event == "transcribe.complete" {
            return event
                .payload
                .get("transcriptPath")
                .and_then(serde_json::Value::as_str)
                .map(PathBuf::from)
                .ok_or_else(|| "Dictation transcription returned no transcript".to_owned());
        }
    }
    Err("Dictation transcription did not complete".to_owned())
}

#[cfg(test)]
pub(crate) fn transcript_body_for_test(transcript: &str) -> String {
    transcript_body(transcript)
}

fn transcript_body(transcript: &str) -> String {
    transcript
        .lines()
        .skip_while(|line| line.starts_with('#') || line.trim().is_empty())
        .map(strip_timestamp)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

fn strip_timestamp(line: &str) -> &str {
    match line.find("] ") {
        Some(timestamp_end) if line.starts_with('[') => &line[timestamp_end + 2..],
        _ => line,
    }
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|error| format!("Unable to access clipboard: {error}"))?;
    clipboard
        .set_text(text)
        .map_err(|error| format!("Unable to copy dictation: {error}"))
}

fn emit_update(app: &tauri::AppHandle, update: &DictationStateUpdate) {
    let _ = app.emit(DICTATION_STATE_EVENT, update.clone());
}

fn new_session_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("dictation-{timestamp}")
}
