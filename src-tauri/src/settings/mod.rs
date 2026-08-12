pub(crate) mod recommendation;

use std::path::Path;

use crate::domain::types::*;
use crate::utils::{default_model_storage_root, default_records_root, option_number_to_string};
pub(crate) const DEFAULT_SUMMARY_PROMPT: &str =
    "Summarize the conversation. Then provide bullet lists for decisions and action items.";
pub(crate) const SUMMARY_PROVIDER_API_KEY_SETTING: &str = "providerApiKey";
pub(crate) const HUGGING_FACE_TOKEN_SETTING: &str = "huggingFaceToken";
const SUPPORTED_WHISPER_MODELS: [&str; 4] = ["small", "medium", "large-v3", "distil-large-v3"];
const SUPPORTED_TRANSCRIPTION_LANGUAGES: [&str; 5] = ["auto", "en", "ru", "uk", "es"];
const DEFAULT_DICTATION_LANGUAGE: &str = "en";

pub(crate) fn default_settings(database_path: &Path) -> AppSettings {
    AppSettings {
        output_directory: default_records_root(),
        database_path: database_path.display().to_string(),
        hotkey: "CommandOrControl+Shift+Space".to_owned(),
        overlay_position: OverlayPosition::TopLeft,
        overlay_display_mode: OverlayDisplayMode::Full,
        dictation_hotkey: "CommandOrControl+Shift+D".to_owned(),
        dictation_shortcut_mode: DictationShortcutMode::Toggle,
        dictation_whisper_model: "small".to_owned(),
        dictation_language: DEFAULT_DICTATION_LANGUAGE.to_owned(),
        dictation_context: String::new(),
        dictation_overlay_position: OverlayPosition::TopRight,
        dictation_overlay_display_mode: OverlayDisplayMode::Minimal,
        close_to_tray: true,
        launch_at_login: false,
        microphone_device: "Default microphone".to_owned(),
        system_audio_source: "Default system output".to_owned(),
        sample_rate: 48_000,
        whisper_model: "small".to_owned(),
        model_recommendation: ModelRecommendation {
            recommended_model: "small".to_owned(),
            reason: "Small is the safest default until hardware capabilities are checked"
                .to_owned(),
            user_overridden: false,
        },
        transcription_language: "auto".to_owned(),
        transcription_context: String::new(),
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
        summary_prompt: DEFAULT_SUMMARY_PROMPT.to_owned(),
    }
}

pub(crate) fn default_model_inventory() -> Vec<ModelInventoryItem> {
    SUPPORTED_WHISPER_MODELS
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
        ("dictationHotkey", input.dictation_hotkey.clone()),
        (
            "dictationShortcutMode",
            serde_json::to_string(&input.dictation_shortcut_mode).unwrap_or_default(),
        ),
        (
            "dictationWhisperModel",
            input.dictation_whisper_model.clone(),
        ),
        ("dictationLanguage", input.dictation_language.clone()),
        ("dictationContext", input.dictation_context.clone()),
        (
            "dictationOverlayPosition",
            serde_json::to_string(&input.dictation_overlay_position).unwrap_or_default(),
        ),
        (
            "dictationOverlayDisplayMode",
            serde_json::to_string(&input.dictation_overlay_display_mode).unwrap_or_default(),
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
        ("transcriptionContext", input.transcription_context.clone()),
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
        ("summaryPrompt", input.summary_prompt.clone()),
    ]
}

pub(crate) fn validate_settings(
    input: &AppSettingsUpdate,
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

    validate_hotkey(&input.hotkey, "Hotkey")?;
    validate_hotkey(&input.dictation_hotkey, "Dictation hotkey")?;

    if input.hotkey == input.dictation_hotkey {
        return Err(rusqlite::Error::InvalidParameterName(
            "Meeting and dictation shortcuts must be different".to_owned(),
        ));
    }

    if !SUPPORTED_WHISPER_MODELS.contains(&input.whisper_model.as_str())
        || !SUPPORTED_WHISPER_MODELS.contains(&input.dictation_whisper_model.as_str())
    {
        return Err(rusqlite::Error::InvalidParameterName(
            "Unsupported Whisper model".to_owned(),
        ));
    }

    if !SUPPORTED_TRANSCRIPTION_LANGUAGES.contains(&input.transcription_language.as_str())
        || input.dictation_language == "auto"
        || !SUPPORTED_TRANSCRIPTION_LANGUAGES.contains(&input.dictation_language.as_str())
    {
        return Err(rusqlite::Error::InvalidParameterName(
            "Unsupported transcription language".to_owned(),
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

fn validate_hotkey(hotkey: &str, label: &str) -> rusqlite::Result<()> {
    let parts = hotkey.split('+').collect::<Vec<_>>();
    let key = parts.last().copied().unwrap_or_default();
    let modifiers = &parts[..parts.len().saturating_sub(1)];
    let valid_modifiers = modifiers
        .iter()
        .all(|part| matches!(*part, "CommandOrControl" | "Alt" | "Shift"));
    let valid_key = key == "Space"
        || (key.chars().count() == 1
            && key.chars().all(|character| {
                character.is_ascii_alphanumeric() || character.is_ascii_punctuation()
            }));

    if parts.is_empty()
        || modifiers.len() > 3
        || !valid_modifiers
        || !valid_key
        || hotkey.trim() != hotkey
    {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "{label} is not a supported shortcut"
        )));
    }

    Ok(())
}

pub(crate) fn summary_provider_configured_for(
    summary_enabled: bool,
    provider_base_url: &str,
    provider_model: &str,
) -> bool {
    summary_enabled && !provider_base_url.trim().is_empty() && !provider_model.trim().is_empty()
}
