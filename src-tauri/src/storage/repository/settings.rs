use rusqlite::{params, OptionalExtension};

use crate::domain::types::*;
use crate::settings::{
    settings_pairs, summary_provider_configured_for, validate_settings, DEFAULT_SUMMARY_PROMPT,
    HUGGING_FACE_TOKEN_SETTING, SUMMARY_PROVIDER_API_KEY_SETTING,
};
use crate::storage::repository::AppRepository;
use crate::utils::{
    default_model_storage_root, default_records_root, ensure_configured_storage_directories,
    parse_bool, parse_optional_number,
};

impl AppRepository {
    pub(crate) fn settings(&self) -> rusqlite::Result<AppSettings> {
        let mut rows = self.connection.prepare("SELECT key, value FROM settings")?;
        let setting_pairs = rows.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut values = Vec::new();

        for pair in setting_pairs {
            values.push(pair?);
        }

        let get_value = |key: &str, fallback: &str| -> String {
            for (candidate_key, value) in &values {
                if candidate_key == key {
                    return value.clone();
                }
            }

            fallback.to_owned()
        };
        let summary_enabled = parse_bool(&get_value("summaryEnabled", "false"));
        let provider_base_url = get_value("providerBaseUrl", "https://api.openai.com/v1");
        let provider_model = get_value("providerModel", "");
        let provider_api_key_configured =
            secret_configured(&get_value(SUMMARY_PROVIDER_API_KEY_SETTING, ""));
        let hugging_face_token_configured =
            secret_configured(&get_value(HUGGING_FACE_TOKEN_SETTING, ""));
        let summary_provider_configured =
            summary_provider_configured_for(summary_enabled, &provider_base_url, &provider_model);

        Ok(AppSettings {
            output_directory: get_value("outputDirectory", &default_records_root()),
            database_path: self.database_path.display().to_string(),
            hotkey: get_value("hotkey", "CommandOrControl+Shift+Space"),
            overlay_position: serde_json::from_str(&get_value("overlayPosition", "\"topLeft\""))
                .unwrap_or(OverlayPosition::TopLeft),
            overlay_display_mode: serde_json::from_str(&get_value(
                "overlayDisplayMode",
                "\"full\"",
            ))
            .unwrap_or(OverlayDisplayMode::Full),
            close_to_tray: parse_bool(&get_value("closeToTray", "true")),
            launch_at_login: parse_bool(&get_value("launchAtLogin", "false")),
            microphone_device: get_value("microphoneDevice", "Default microphone"),
            system_audio_source: get_value("systemAudioSource", "Default system output"),
            sample_rate: get_value("sampleRate", "48000").parse().unwrap_or(48_000),
            whisper_model: get_value("whisperModel", "medium"),
            transcription_language: get_value("transcriptionLanguage", "auto"),
            compute_type: get_value("computeType", "auto"),
            model_storage_directory: get_value(
                "modelStorageDirectory",
                &default_model_storage_root(),
            ),
            diarization_backend: serde_json::from_str(&get_value(
                "diarizationBackend",
                "\"sortformer\"",
            ))
            .unwrap_or(DiarizationBackend::Sortformer),
            speaker_count_mode: serde_json::from_str(&get_value(
                "speakerCountMode",
                "\"automatic\"",
            ))
            .unwrap_or(SpeakerCountMode::Automatic),
            exact_speakers: parse_optional_number(&get_value("exactSpeakers", "")),
            min_speakers: parse_optional_number(&get_value("minSpeakers", "")),
            max_speakers: parse_optional_number(&get_value("maxSpeakers", "")),
            hugging_face_token_configured,
            diarization_setup_skipped: parse_bool(&get_value("diarizationSetupSkipped", "false")),
            diarization_runtime_ready: parse_bool(&get_value("diarizationRuntimeReady", "false")),
            summary_provider_configured,
            provider_api_key_configured,
            summary_enabled,
            provider_base_url,
            provider_model,
            summary_prompt: get_value("summaryPrompt", DEFAULT_SUMMARY_PROMPT),
        })
    }

    pub(crate) fn update_settings(&mut self, input: AppSettingsUpdate) -> rusqlite::Result<()> {
        let cuda_available = self.desktop_runtime_status()?.cuda_available;
        let provider_api_key = self.updated_secret(
            SUMMARY_PROVIDER_API_KEY_SETTING,
            input.provider_api_key.as_deref(),
        )?;
        let hugging_face_token = self.updated_secret(
            HUGGING_FACE_TOKEN_SETTING,
            input.hugging_face_token.as_deref(),
        )?;
        let provider_api_key_configured = secret_configured(&provider_api_key);
        let hugging_face_token_configured = secret_configured(&hugging_face_token);

        validate_settings(&input, cuda_available)?;
        ensure_configured_storage_directories(
            &input.output_directory,
            &input.model_storage_directory,
        )?;

        let transaction = self.connection.transaction()?;
        let provider_configured = summary_provider_configured_for(
            input.summary_enabled,
            &input.provider_base_url,
            &input.provider_model,
        );

        for (key, value) in settings_pairs(
            &input,
            provider_configured,
            provider_api_key_configured,
            hugging_face_token_configured,
        ) {
            transaction.execute(
                "
                INSERT INTO settings (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                ",
                params![key, value],
            )?;
        }
        for (key, value) in [
            (SUMMARY_PROVIDER_API_KEY_SETTING, provider_api_key),
            (HUGGING_FACE_TOKEN_SETTING, hugging_face_token),
        ] {
            transaction.execute(
                "
                INSERT INTO settings (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                ",
                params![key, value],
            )?;
        }

        transaction.execute(
            "
            INSERT INTO providers (id, name, base_url, model, enabled)
            VALUES ('summary', 'OpenAI-compatible summary', ?1, ?2, ?3)
            ON CONFLICT(id) DO UPDATE SET
                base_url = excluded.base_url,
                model = excluded.model,
                enabled = excluded.enabled
            ",
            params![
                input.provider_base_url,
                input.provider_model,
                provider_configured
            ],
        )?;
        transaction.commit()
    }

    pub(crate) fn read_summary_provider_api_key(&self) -> rusqlite::Result<Option<String>> {
        self.read_secret(SUMMARY_PROVIDER_API_KEY_SETTING)
    }

    pub(crate) fn read_hugging_face_token(&self) -> rusqlite::Result<Option<String>> {
        self.read_secret(HUGGING_FACE_TOKEN_SETTING)
    }

    pub(crate) fn clear_summary_provider_api_key(&mut self) -> rusqlite::Result<()> {
        self.set_setting(SUMMARY_PROVIDER_API_KEY_SETTING, "")?;
        self.update_summary_provider_status(false)
    }

    pub(crate) fn clear_hugging_face_token(&mut self) -> rusqlite::Result<()> {
        self.set_setting(HUGGING_FACE_TOKEN_SETTING, "")?;
        self.update_hugging_face_token_status(false)
    }

    pub(crate) fn update_hugging_face_token(
        &mut self,
        token: Option<&str>,
    ) -> rusqlite::Result<bool> {
        let updated_token = self.updated_secret(HUGGING_FACE_TOKEN_SETTING, token)?;
        let configured = secret_configured(&updated_token);

        self.set_setting(HUGGING_FACE_TOKEN_SETTING, &updated_token)?;
        self.update_hugging_face_token_status(configured)?;

        Ok(configured)
    }

    pub(crate) fn ensure_current_storage_directories(&self) -> rusqlite::Result<()> {
        let settings = self.settings()?;

        ensure_configured_storage_directories(
            &settings.output_directory,
            &settings.model_storage_directory,
        )
    }

    pub(crate) fn update_summary_provider_status(
        &mut self,
        provider_api_key_configured: bool,
    ) -> rusqlite::Result<()> {
        let settings = self.settings()?;
        let provider_configured = summary_provider_configured_for(
            settings.summary_enabled,
            &settings.provider_base_url,
            &settings.provider_model,
        );
        let transaction = self.connection.transaction()?;

        for (key, value) in [
            ("summaryProviderConfigured", provider_configured.to_string()),
            (
                "providerApiKeyConfigured",
                provider_api_key_configured.to_string(),
            ),
        ] {
            transaction.execute(
                "
                INSERT INTO settings (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                ",
                params![key, value],
            )?;
        }

        transaction.execute(
            "
            UPDATE providers
            SET enabled = ?1
            WHERE id = 'summary'
            ",
            params![provider_configured],
        )?;
        transaction.commit()
    }

    pub(crate) fn update_hugging_face_token_status(
        &mut self,
        configured: bool,
    ) -> rusqlite::Result<()> {
        self.set_setting("huggingFaceTokenConfigured", &configured.to_string())
    }

    pub(crate) fn update_diarization_setup_skipped(
        &mut self,
        skipped: bool,
    ) -> rusqlite::Result<()> {
        self.set_setting("diarizationSetupSkipped", &skipped.to_string())
    }

    pub(crate) fn update_diarization_runtime_ready(&mut self, ready: bool) -> rusqlite::Result<()> {
        self.set_setting("diarizationRuntimeReady", &ready.to_string())
    }

    pub(crate) fn set_setting(&mut self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            INSERT INTO settings (key, value)
            VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            ",
            params![key, value],
        )?;

        Ok(())
    }

    pub(crate) fn setting_value(&self, key: &str, fallback: &str) -> rusqlite::Result<String> {
        self.connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map(|value| value.unwrap_or_else(|| fallback.to_owned()))
    }

    pub(crate) fn read_secret(&self, key: &str) -> rusqlite::Result<Option<String>> {
        let value = self.setting_value(key, "")?;

        Ok(secret_configured(&value).then_some(value))
    }

    pub(crate) fn updated_secret(
        &self,
        key: &str,
        input: Option<&str>,
    ) -> rusqlite::Result<String> {
        let input = input.unwrap_or_default().trim();

        if input.is_empty() {
            return self.setting_value(key, "");
        }

        Ok(input.to_owned())
    }
}

fn secret_configured(value: &str) -> bool {
    !value.trim().is_empty()
}
