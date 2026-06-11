use std::path::Path;

use crate::domain::types::*;
use crate::utils::{default_model_storage_root, default_records_root, option_number_to_string};
pub(crate) const DEFAULT_TITLE_PROMPT: &str = "Create a concise meeting title from the transcript.";
pub(crate) const DEFAULT_SUMMARY_PROMPT: &str =
    "Summarize decisions, action items, risks, and unanswered questions.";
pub(crate) const SUMMARY_PROVIDER_API_KEY_SETTING: &str = "providerApiKey";
pub(crate) const HUGGING_FACE_TOKEN_SETTING: &str = "huggingFaceToken";

pub(crate) fn default_settings(database_path: &Path) -> AppSettings {
    AppSettings {
        output_directory: default_records_root(),
        database_path: database_path.display().to_string(),
        hotkey: "CommandOrControl+Shift+Space".to_owned(),
        overlay_position: OverlayPosition::TopLeft,
        overlay_display_mode: OverlayDisplayMode::Full,
        close_to_tray: true,
        launch_at_login: false,
        microphone_device: "Default microphone".to_owned(),
        system_audio_source: "Default system output".to_owned(),
        sample_rate: 48_000,
        whisper_model: "medium".to_owned(),
        transcription_language: "auto".to_owned(),
        compute_type: "auto".to_owned(),
        model_storage_directory: default_model_storage_root(),
        diarization_backend: DiarizationBackend::Sortformer,
        speaker_count_mode: SpeakerCountMode::Automatic,
        exact_speakers: None,
        min_speakers: None,
        max_speakers: None,
        hugging_face_token_configured: false,
        diarization_setup_skipped: false,
        diarization_runtime_ready: false,
        summary_provider_configured: false,
        provider_api_key_configured: false,
        summary_enabled: false,
        provider_base_url: "https://api.openai.com/v1".to_owned(),
        provider_model: String::new(),
        title_prompt: DEFAULT_TITLE_PROMPT.to_owned(),
        summary_prompt: DEFAULT_SUMMARY_PROMPT.to_owned(),
    }
}

pub(crate) fn default_model_inventory() -> Vec<ModelInventoryItem> {
    ["small", "medium", "large-v3", "distil-large-v3"]
        .iter()
        .map(|model| ModelInventoryItem {
            name: (*model).to_owned(),
            installed: false,
            setup_required: true,
            dependency: "faster-whisper".to_owned(),
        })
        .collect()
}

pub(crate) fn settings_pairs(
    input: &AppSettingsUpdate,
    summary_provider_configured: bool,
    provider_api_key_configured: bool,
    hugging_face_token_configured: bool,
) -> Vec<(&'static str, String)> {
    vec![
        ("outputDirectory", input.output_directory.clone()),
        ("hotkey", input.hotkey.clone()),
        (
            "overlayPosition",
            serde_json::to_string(&input.overlay_position).unwrap_or_default(),
        ),
        (
            "overlayDisplayMode",
            serde_json::to_string(&input.overlay_display_mode).unwrap_or_default(),
        ),
        ("closeToTray", input.close_to_tray.to_string()),
        ("launchAtLogin", input.launch_at_login.to_string()),
        ("microphoneDevice", input.microphone_device.clone()),
        ("systemAudioSource", input.system_audio_source.clone()),
        ("sampleRate", input.sample_rate.to_string()),
        ("whisperModel", input.whisper_model.clone()),
        (
            "transcriptionLanguage",
            input.transcription_language.clone(),
        ),
        ("computeType", input.compute_type.clone()),
        (
            "modelStorageDirectory",
            input.model_storage_directory.clone(),
        ),
        (
            "diarizationBackend",
            serde_json::to_string(&input.diarization_backend).unwrap_or_default(),
        ),
        (
            "speakerCountMode",
            serde_json::to_string(&input.speaker_count_mode).unwrap_or_default(),
        ),
        (
            "exactSpeakers",
            option_number_to_string(input.exact_speakers),
        ),
        ("minSpeakers", option_number_to_string(input.min_speakers)),
        ("maxSpeakers", option_number_to_string(input.max_speakers)),
        (
            "huggingFaceTokenConfigured",
            hugging_face_token_configured.to_string(),
        ),
        (
            "diarizationSetupSkipped",
            input.diarization_setup_skipped.to_string(),
        ),
        (
            "summaryProviderConfigured",
            summary_provider_configured.to_string(),
        ),
        (
            "providerApiKeyConfigured",
            provider_api_key_configured.to_string(),
        ),
        ("summaryEnabled", input.summary_enabled.to_string()),
        ("providerBaseUrl", input.provider_base_url.clone()),
        ("providerModel", input.provider_model.clone()),
        ("titlePrompt", input.title_prompt.clone()),
        ("summaryPrompt", input.summary_prompt.clone()),
    ]
}

pub(crate) fn validate_settings(
    input: &AppSettingsUpdate,
    provider_api_key_configured: bool,
    cuda_available: bool,
) -> rusqlite::Result<()> {
    if input.output_directory.trim().is_empty() {
        return Err(rusqlite::Error::InvalidParameterName(
            "Output directory is required".to_owned(),
        ));
    }

    if input.hotkey.trim().is_empty() {
        return Err(rusqlite::Error::InvalidParameterName(
            "Hotkey is required".to_owned(),
        ));
    }

    if input.sample_rate == 0 {
        return Err(rusqlite::Error::InvalidParameterName(
            "Sample rate must be greater than zero".to_owned(),
        ));
    }

    if input.compute_type == "cuda" && !cuda_available {
        return Err(rusqlite::Error::InvalidParameterName(
            "CUDA runtime is not ready. Install CUDA drivers, cuBLAS for CUDA 12, and cuDNN 9 for CUDA 12"
                .to_owned(),
        ));
    }

    if input.summary_enabled {
        if input.provider_base_url.trim().is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Provider base URL is required when summaries are enabled".to_owned(),
            ));
        }

        if input.provider_model.trim().is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Provider model is required when summaries are enabled".to_owned(),
            ));
        }

        if !provider_api_key_configured {
            return Err(rusqlite::Error::InvalidParameterName(
                "Provider API key is required when summaries are enabled".to_owned(),
            ));
        }
    }

    match input.speaker_count_mode {
        SpeakerCountMode::Automatic => Ok(()),
        SpeakerCountMode::Exact => match input.exact_speakers {
            Some(value) if value > 0 => Ok(()),
            _ => Err(rusqlite::Error::InvalidParameterName(
                "Exact speaker count must be greater than zero".to_owned(),
            )),
        },
        SpeakerCountMode::Range => match (input.min_speakers, input.max_speakers) {
            (Some(min), Some(max)) if min > 0 && max >= min => Ok(()),
            _ => Err(rusqlite::Error::InvalidParameterName(
                "Speaker range must include a valid minimum and maximum".to_owned(),
            )),
        },
    }
}

pub(crate) fn summary_provider_configured_for(
    summary_enabled: bool,
    provider_base_url: &str,
    provider_model: &str,
    provider_api_key_configured: bool,
) -> bool {
    summary_enabled
        && !provider_base_url.trim().is_empty()
        && !provider_model.trim().is_empty()
        && provider_api_key_configured
}
