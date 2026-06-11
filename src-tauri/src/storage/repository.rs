use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::artifacts::{recording_stages, stage_label, stage_message};
use crate::capture::audio::capture_devices;
use crate::domain::types::*;
use crate::settings::{
    default_model_inventory, default_settings, settings_pairs, summary_provider_configured_for,
    validate_settings, DEFAULT_SUMMARY_PROMPT, DEFAULT_TITLE_PROMPT, HUGGING_FACE_TOKEN_SETTING,
    SUMMARY_PROVIDER_API_KEY_SETTING,
};
use crate::utils::{
    default_model_storage_root, default_records_root, empty_string_to_none,
    ensure_configured_storage_directories, enum_from_value, enum_value, json_string,
    option_number_to_string, parse_bool, parse_optional_number, remove_artifact_directory,
    row_to_pipeline_job, unix_timestamp,
};
#[derive(Debug)]
pub(crate) struct AppRepository {
    pub(crate) connection: Connection,
    pub(crate) database_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NewRecording {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) started_at: String,
    pub(crate) artifact_directory: String,
}

impl AppRepository {
    pub(crate) fn open(database_path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }

        let connection = Connection::open(database_path)?;
        let repository = Self {
            connection,
            database_path: database_path.to_path_buf(),
        };

        repository.migrate()?;
        repository.seed_defaults()?;

        Ok(repository)
    }

    pub(crate) fn migrate(&self) -> rusqlite::Result<()> {
        self.connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS recordings (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                duration_seconds INTEGER,
                status TEXT NOT NULL,
                artifact_directory TEXT NOT NULL,
                capture_errors TEXT NOT NULL DEFAULT '[]'
            );

            CREATE TABLE IF NOT EXISTS recording_artifacts (
                recording_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                label TEXT NOT NULL,
                path TEXT NOT NULL,
                ready INTEGER NOT NULL,
                PRIMARY KEY (recording_id, kind),
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS pipeline_jobs (
                id TEXT PRIMARY KEY,
                recording_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                status TEXT NOT NULL,
                progress INTEGER NOT NULL,
                message TEXT NOT NULL,
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_url TEXT NOT NULL,
                model TEXT NOT NULL,
                enabled INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS models (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                name TEXT NOT NULL,
                installed INTEGER NOT NULL,
                setup_required INTEGER NOT NULL DEFAULT 1,
                dependency TEXT NOT NULL DEFAULT 'faster-whisper'
            );

            CREATE TABLE IF NOT EXISTS job_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                recording_id TEXT NOT NULL,
                stage TEXT NOT NULL,
                status TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

            INSERT OR IGNORE INTO schema_migrations (version, applied_at)
            VALUES (1, strftime('%s', 'now'));
            ",
        )?;
        self.ensure_column(
            "models",
            "setup_required",
            "ALTER TABLE models ADD COLUMN setup_required INTEGER NOT NULL DEFAULT 1",
        )?;
        self.ensure_column(
            "models",
            "dependency",
            "ALTER TABLE models ADD COLUMN dependency TEXT NOT NULL DEFAULT 'faster-whisper'",
        )
    }

    pub(crate) fn ensure_column(
        &self,
        table: &str,
        column: &str,
        sql: &str,
    ) -> rusqlite::Result<()> {
        let mut statement = self
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = statement.query_map([], |row| row.get::<_, String>(1))?;

        for existing_column in rows {
            if existing_column? == column {
                return Ok(());
            }
        }

        self.connection.execute(sql, [])?;

        Ok(())
    }

    pub(crate) fn seed_defaults(&self) -> rusqlite::Result<()> {
        let settings = default_settings(&self.database_path);

        self.upsert_setting("outputDirectory", &settings.output_directory)?;
        self.upsert_setting("databasePath", &settings.database_path)?;
        self.upsert_setting("hotkey", &settings.hotkey)?;
        self.upsert_setting("overlayPosition", &json_string(&settings.overlay_position)?)?;
        self.upsert_setting(
            "overlayDisplayMode",
            &json_string(&settings.overlay_display_mode)?,
        )?;
        self.upsert_setting("closeToTray", &settings.close_to_tray.to_string())?;
        self.upsert_setting("launchAtLogin", &settings.launch_at_login.to_string())?;
        self.upsert_setting("microphoneDevice", &settings.microphone_device)?;
        self.upsert_setting("systemAudioSource", &settings.system_audio_source)?;
        self.upsert_setting("sampleRate", &settings.sample_rate.to_string())?;
        self.upsert_setting("whisperModel", &settings.whisper_model)?;
        self.upsert_setting("transcriptionLanguage", &settings.transcription_language)?;
        self.upsert_setting("computeType", &settings.compute_type)?;
        self.upsert_setting("modelStorageDirectory", &settings.model_storage_directory)?;
        self.upsert_setting(
            "diarizationBackend",
            &json_string(&settings.diarization_backend)?,
        )?;
        self.upsert_setting(
            "speakerCountMode",
            &json_string(&settings.speaker_count_mode)?,
        )?;
        self.upsert_setting(
            "exactSpeakers",
            &option_number_to_string(settings.exact_speakers),
        )?;
        self.upsert_setting(
            "minSpeakers",
            &option_number_to_string(settings.min_speakers),
        )?;
        self.upsert_setting(
            "maxSpeakers",
            &option_number_to_string(settings.max_speakers),
        )?;
        self.upsert_setting(
            "summaryProviderConfigured",
            &settings.summary_provider_configured.to_string(),
        )?;
        self.upsert_setting(
            "providerApiKeyConfigured",
            &settings.provider_api_key_configured.to_string(),
        )?;
        self.upsert_setting(SUMMARY_PROVIDER_API_KEY_SETTING, "")?;
        self.upsert_setting(
            "huggingFaceTokenConfigured",
            &settings.hugging_face_token_configured.to_string(),
        )?;
        self.upsert_setting(HUGGING_FACE_TOKEN_SETTING, "")?;
        self.upsert_setting(
            "diarizationSetupSkipped",
            &settings.diarization_setup_skipped.to_string(),
        )?;
        self.upsert_setting(
            "diarizationRuntimeReady",
            &settings.diarization_runtime_ready.to_string(),
        )?;
        self.upsert_setting("overlayVisible", "false")?;
        self.upsert_setting("hotkeyRegistered", "false")?;
        self.upsert_setting("hotkeyError", "")?;
        self.upsert_setting("workerRunning", "false")?;
        self.upsert_setting("workerHealthOk", "false")?;
        self.upsert_setting("workerError", "")?;
        self.upsert_setting(
            "workerSetupStatus",
            &enum_value(WorkerSetupStatus::Missing)?,
        )?;
        self.upsert_setting("workerSetupStep", "")?;
        self.upsert_setting("workerSetupError", "")?;
        self.upsert_setting("cudaAvailable", "false")?;
        self.upsert_setting("cudaError", "")?;
        self.upsert_setting("summaryEnabled", &settings.summary_enabled.to_string())?;
        self.upsert_setting("providerBaseUrl", &settings.provider_base_url)?;
        self.upsert_setting("providerModel", &settings.provider_model)?;
        self.upsert_setting("titlePrompt", &settings.title_prompt)?;
        self.upsert_setting("summaryPrompt", &settings.summary_prompt)?;
        self.migrate_default_whisper_model()?;
        self.seed_default_models()
    }

    pub(crate) fn upsert_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            INSERT OR IGNORE INTO settings (key, value)
            VALUES (?1, ?2)
            ",
            params![key, value],
        )?;

        Ok(())
    }

    pub(crate) fn migrate_default_whisper_model(&self) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            UPDATE settings
            SET value = 'medium'
            WHERE key = 'whisperModel'
                AND value IN ('small.en', 'medium.en')
            ",
            [],
        )?;

        Ok(())
    }

    pub(crate) fn seed_default_models(&self) -> rusqlite::Result<()> {
        for model in default_model_inventory() {
            self.connection.execute(
                "
                INSERT OR IGNORE INTO models (id, provider, name, installed, setup_required, dependency)
                VALUES (?1, 'faster-whisper', ?2, ?3, ?4, ?5)
                ",
                params![
                    model.name,
                    model.name,
                    model.installed,
                    model.setup_required,
                    model.dependency,
                ],
            )?;
        }

        Ok(())
    }

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
        let summary_provider_configured = summary_provider_configured_for(
            summary_enabled,
            &provider_base_url,
            &provider_model,
            provider_api_key_configured,
        );

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
            title_prompt: get_value("titlePrompt", DEFAULT_TITLE_PROMPT),
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

        validate_settings(&input, provider_api_key_configured, cuda_available)?;
        ensure_configured_storage_directories(
            &input.output_directory,
            &input.model_storage_directory,
        )?;

        let transaction = self.connection.transaction()?;
        let provider_configured = summary_provider_configured_for(
            input.summary_enabled,
            &input.provider_base_url,
            &input.provider_model,
            provider_api_key_configured,
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
            provider_api_key_configured,
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

    pub(crate) fn update_desktop_runtime_status(
        &mut self,
        status: &DesktopRuntimeStatus,
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;
        let values = [
            ("overlayVisible", status.overlay_visible.to_string()),
            ("hotkeyRegistered", status.hotkey_registered.to_string()),
            (
                "hotkeyError",
                status.hotkey_error.clone().unwrap_or_default(),
            ),
            ("workerRunning", status.worker_running.to_string()),
            ("workerHealthOk", status.worker_health_ok.to_string()),
            (
                "workerError",
                status.worker_error.clone().unwrap_or_default(),
            ),
            ("workerSetupStatus", enum_value(status.worker_setup_status)?),
            ("workerSetupStep", status.worker_setup_step.clone()),
            (
                "workerSetupError",
                status.worker_setup_error.clone().unwrap_or_default(),
            ),
            ("cudaAvailable", status.cuda_available.to_string()),
            ("cudaError", status.cuda_error.clone().unwrap_or_default()),
        ];

        for (key, value) in values {
            transaction.execute(
                "
                INSERT INTO settings (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                ",
                params![key, value],
            )?;
        }

        transaction.commit()
    }

    pub(crate) fn update_worker_runtime_status(
        &mut self,
        status: &WorkerStatus,
    ) -> rusqlite::Result<()> {
        let mut desktop_status = self.desktop_runtime_status()?;

        desktop_status.worker_running = status.running;
        desktop_status.worker_health_ok = status.health_ok;
        desktop_status.worker_error = status.last_error.clone();

        self.update_desktop_runtime_status(&desktop_status)
    }

    pub(crate) fn update_runtime_capabilities(
        &mut self,
        capabilities: &RuntimeCapabilities,
    ) -> rusqlite::Result<()> {
        let mut desktop_status = self.desktop_runtime_status()?;

        desktop_status.cuda_available = capabilities.cuda_available;
        desktop_status.cuda_error = capabilities.cuda_error.clone();

        self.update_desktop_runtime_status(&desktop_status)
    }

    pub(crate) fn update_worker_setup_progress(
        &mut self,
        progress: &WorkerSetupProgress,
    ) -> rusqlite::Result<()> {
        let mut desktop_status = self.desktop_runtime_status()?;

        desktop_status.worker_setup_status = progress.status;
        desktop_status.worker_setup_step = progress.step.clone();
        desktop_status.worker_setup_error = progress.error.clone();

        match progress.status {
            WorkerSetupStatus::Ready => {
                desktop_status.worker_running = true;
                desktop_status.worker_health_ok = true;
                desktop_status.worker_error = None;
            }
            WorkerSetupStatus::Failed => {
                desktop_status.worker_running = true;
                desktop_status.worker_health_ok = false;
                desktop_status.worker_error = progress.error.clone();
            }
            WorkerSetupStatus::Missing | WorkerSetupStatus::Installing => (),
        }

        self.update_desktop_runtime_status(&desktop_status)
    }

    pub(crate) fn set_worker_error(&mut self, message: &str) -> rusqlite::Result<()> {
        let mut desktop_status = self.desktop_runtime_status()?;

        desktop_status.worker_running = true;
        desktop_status.worker_health_ok = false;
        desktop_status.worker_error = Some(message.to_owned());

        self.update_desktop_runtime_status(&desktop_status)
    }

    pub(crate) fn clear_worker_error(&mut self) -> rusqlite::Result<()> {
        let mut desktop_status = self.desktop_runtime_status()?;

        desktop_status.worker_running = true;
        desktop_status.worker_health_ok = true;
        desktop_status.worker_error = None;

        self.update_desktop_runtime_status(&desktop_status)
    }

    pub(crate) fn replace_model_inventory(
        &mut self,
        models: &[ModelInventoryItem],
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;

        for model in models {
            transaction.execute(
                "
                INSERT INTO models (id, provider, name, installed, setup_required, dependency)
                VALUES (?1, 'faster-whisper', ?2, ?3, ?4, ?5)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    installed = excluded.installed,
                    setup_required = excluded.setup_required,
                    dependency = excluded.dependency
                ",
                params![
                    model.name,
                    model.name,
                    model.installed,
                    model.setup_required,
                    model.dependency,
                ],
            )?;
        }

        transaction.commit()
    }

    pub(crate) fn active_recording(&self) -> rusqlite::Result<Option<Recording>> {
        self.recording_by_status(RecordingStatus::Recording)
    }

    pub(crate) fn clear_stale_active_recordings(&mut self) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;
        let ended_at = unix_timestamp().to_string();

        transaction.execute(
            "
            UPDATE pipeline_jobs
            SET status = ?1,
                progress = 0,
                message = ?2
            WHERE stage = ?3
                AND recording_id IN (
                    SELECT id
                    FROM recordings
                    WHERE status = ?4
                )
            ",
            params![
                enum_value(PipelineStageStatus::Failed)?,
                "Capture session was interrupted",
                enum_value(PipelineStageId::Recording)?,
                enum_value(RecordingStatus::Recording)?,
            ],
        )?;

        transaction.execute(
            "
            UPDATE recordings
            SET ended_at = COALESCE(ended_at, ?1),
                duration_seconds = COALESCE(duration_seconds, 0),
                status = ?2
            WHERE status = ?3
            ",
            params![
                ended_at,
                enum_value(RecordingStatus::Idle)?,
                enum_value(RecordingStatus::Recording)?,
            ],
        )?;

        transaction.commit()
    }

    pub(crate) fn create_recording(&mut self, recording: NewRecording) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;

        transaction.execute(
            "
            INSERT INTO recordings (
                id,
                title,
                started_at,
                ended_at,
                duration_seconds,
                status,
                artifact_directory,
                capture_errors
            )
            VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?5, '[]')
            ",
            params![
                recording.id,
                recording.title,
                recording.started_at,
                enum_value(RecordingStatus::Recording)?,
                recording.artifact_directory
            ],
        )?;

        for stage in recording_stages() {
            transaction.execute(
                "
                INSERT INTO pipeline_jobs (id, recording_id, stage, status, progress, message)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    format!("{}-{}", recording.id, enum_value(stage.id)?),
                    recording.id,
                    enum_value(stage.id)?,
                    enum_value(stage.status)?,
                    stage.progress,
                    stage.label,
                ],
            )?;
        }

        transaction.commit()
    }

    pub(crate) fn finish_recording(
        &mut self,
        recording_id: &str,
        ended_at: String,
        duration_seconds: u64,
        capture_errors: Vec<CaptureError>,
        artifacts: &[Artifact],
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;
        let capture_errors = json_string(&capture_errors)?;

        transaction.execute(
            "
            UPDATE recordings
            SET ended_at = ?1,
                duration_seconds = ?2,
                status = ?3,
                capture_errors = ?4
            WHERE id = ?5
            ",
            params![
                ended_at,
                duration_seconds,
                enum_value(RecordingStatus::Processing)?,
                capture_errors,
                recording_id,
            ],
        )?;

        for artifact in artifacts {
            transaction.execute(
                "
                INSERT INTO recording_artifacts (recording_id, kind, label, path, ready)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(recording_id, kind) DO UPDATE SET
                    label = excluded.label,
                    path = excluded.path,
                    ready = excluded.ready
                ",
                params![
                    recording_id,
                    enum_value(artifact.kind)?,
                    artifact.label,
                    artifact.path,
                    artifact.ready,
                ],
            )?;
        }

        let stage_updates = [
            (
                PipelineStageId::Recording,
                PipelineStageStatus::Complete,
                100,
            ),
            (
                PipelineStageId::Transcription,
                PipelineStageStatus::Pending,
                0,
            ),
            (PipelineStageId::Alignment, PipelineStageStatus::Pending, 0),
            (
                PipelineStageId::Diarization,
                PipelineStageStatus::Pending,
                0,
            ),
            (PipelineStageId::Summary, PipelineStageStatus::Pending, 0),
        ];

        for (stage, status, progress) in stage_updates {
            transaction.execute(
                "
                UPDATE pipeline_jobs
                SET status = ?1,
                    progress = ?2,
                    message = ?3
                WHERE recording_id = ?4
                    AND stage = ?5
                ",
                params![
                    enum_value(status)?,
                    progress,
                    stage_message(stage, status),
                    recording_id,
                    enum_value(stage)?,
                ],
            )?;
        }

        transaction.commit()
    }

    pub(crate) fn delete_recording(
        &mut self,
        recording_id: &str,
        delete_artifacts: bool,
    ) -> rusqlite::Result<()> {
        let recording = self.recording_by_id(recording_id)?.ok_or_else(|| {
            rusqlite::Error::InvalidParameterName("Recording not found".to_owned())
        })?;

        if recording.status == RecordingStatus::Recording {
            return Err(rusqlite::Error::InvalidParameterName(
                "Active recordings cannot be deleted".to_owned(),
            ));
        }

        self.connection.execute(
            "
            DELETE FROM recordings
            WHERE id = ?1
            ",
            params![recording_id],
        )?;

        if delete_artifacts {
            remove_artifact_directory(&recording.artifact_directory)?;
        }

        Ok(())
    }

    pub(crate) fn reset_retryable_jobs(&mut self, recording_id: &str) -> rusqlite::Result<()> {
        let recording = self.recording_by_id(recording_id)?.ok_or_else(|| {
            rusqlite::Error::InvalidParameterName("Recording not found".to_owned())
        })?;

        if !matches!(
            recording.status,
            RecordingStatus::Processing | RecordingStatus::Complete
        ) {
            return Err(rusqlite::Error::InvalidParameterName(
                "Only processed recordings can retry jobs".to_owned(),
            ));
        }

        let transaction = self.connection.transaction()?;

        transaction.execute(
            "
            UPDATE recordings
            SET status = ?1
            WHERE id = ?2
            ",
            params![enum_value(RecordingStatus::Processing)?, recording_id],
        )?;

        transaction.execute(
            "
            UPDATE pipeline_jobs
            SET status = ?1,
                progress = 0,
                message = ?2
            WHERE recording_id = ?3
                AND status IN (?4, ?5, ?6)
                AND stage != ?7
            ",
            params![
                enum_value(PipelineStageStatus::Pending)?,
                "Retry queued",
                recording_id,
                enum_value(PipelineStageStatus::Failed)?,
                enum_value(PipelineStageStatus::NeedsSetup)?,
                enum_value(PipelineStageStatus::Running)?,
                enum_value(PipelineStageId::Recording)?,
            ],
        )?;
        transaction.commit()?;

        self.append_event(
            recording_id,
            PipelineStageId::Transcription,
            PipelineStageStatus::Pending,
            "Retry queued",
        )
    }

    pub(crate) fn update_job(
        &mut self,
        job_id: &str,
        status: PipelineStageStatus,
        progress: u8,
        message: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            UPDATE pipeline_jobs
            SET status = ?1,
                progress = ?2,
                message = ?3
            WHERE id = ?4
            ",
            params![enum_value(status)?, progress, message, job_id],
        )?;

        Ok(())
    }

    pub(crate) fn complete_recording_if_pipeline_done(
        &mut self,
        recording_id: &str,
    ) -> rusqlite::Result<()> {
        let unfinished_jobs = self.connection.query_row(
            "
            SELECT COUNT(*)
            FROM pipeline_jobs
            WHERE recording_id = ?1
                AND status NOT IN (?2, ?3)
            ",
            params![
                recording_id,
                enum_value(PipelineStageStatus::Complete)?,
                enum_value(PipelineStageStatus::Skipped)?,
            ],
            |row| row.get::<_, u32>(0),
        )?;

        if unfinished_jobs > 0 {
            return Ok(());
        }

        self.connection.execute(
            "
            UPDATE recordings
            SET status = ?1
            WHERE id = ?2
                AND status = ?3
            ",
            params![
                enum_value(RecordingStatus::Complete)?,
                recording_id,
                enum_value(RecordingStatus::Processing)?,
            ],
        )?;

        Ok(())
    }

    pub(crate) fn job_for_recording_stage(
        &self,
        recording_id: &str,
        stage: PipelineStageId,
    ) -> rusqlite::Result<PipelineJob> {
        self.connection.query_row(
            "
            SELECT id, recording_id, stage, status, progress, message
            FROM pipeline_jobs
            WHERE recording_id = ?1
                AND stage = ?2
            ",
            params![recording_id, enum_value(stage)?],
            row_to_pipeline_job,
        )
    }

    pub(crate) fn upsert_artifact(
        &mut self,
        recording_id: &str,
        artifact: &Artifact,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            INSERT INTO recording_artifacts (recording_id, kind, label, path, ready)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(recording_id, kind) DO UPDATE SET
                label = excluded.label,
                path = excluded.path,
                ready = excluded.ready
            ",
            params![
                recording_id,
                enum_value(artifact.kind)?,
                artifact.label,
                artifact.path,
                artifact.ready,
            ],
        )?;

        Ok(())
    }

    pub(crate) fn update_recording_title(
        &mut self,
        recording_id: &str,
        title: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            UPDATE recordings
            SET title = ?1
            WHERE id = ?2
            ",
            params![title, recording_id],
        )?;

        Ok(())
    }

    pub(crate) fn append_event(
        &mut self,
        recording_id: &str,
        stage: PipelineStageId,
        status: PipelineStageStatus,
        message: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "
            INSERT INTO job_events (recording_id, stage, status, message, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                recording_id,
                enum_value(stage)?,
                enum_value(status)?,
                message,
                unix_timestamp().to_string(),
            ],
        )?;

        Ok(())
    }

    pub(crate) fn snapshot(&self) -> rusqlite::Result<AppSnapshot> {
        let settings = self.settings()?;
        let recordings = self.recordings()?;
        let active_recording = self.active_recording()?;
        let jobs = self.jobs()?;
        let models = self.model_inventory()?;
        let capture_devices = capture_devices();
        let mut desktop = self.desktop_runtime_status()?;

        desktop.overlay_visible =
            active_recording.is_some() && settings.overlay_display_mode != OverlayDisplayMode::None;

        Ok(AppSnapshot {
            active_recording,
            recordings,
            jobs,
            models,
            capture_devices,
            desktop,
            settings,
        })
    }

    pub(crate) fn model_inventory(&self) -> rusqlite::Result<Vec<ModelInventoryItem>> {
        let mut statement = self.connection.prepare(
            "
            SELECT name, installed, setup_required, dependency
            FROM models
            ORDER BY name
            ",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ModelInventoryItem {
                name: row.get(0)?,
                installed: row.get(1)?,
                setup_required: row.get(2)?,
                dependency: row.get(3)?,
            })
        })?;
        let mut models = Vec::new();

        for model in rows {
            models.push(model?);
        }

        Ok(models)
    }

    pub(crate) fn desktop_runtime_status(&self) -> rusqlite::Result<DesktopRuntimeStatus> {
        Ok(DesktopRuntimeStatus {
            overlay_visible: parse_bool(&self.setting_value("overlayVisible", "false")?),
            hotkey_registered: parse_bool(&self.setting_value("hotkeyRegistered", "false")?),
            hotkey_error: empty_string_to_none(self.setting_value("hotkeyError", "")?),
            worker_running: parse_bool(&self.setting_value("workerRunning", "false")?),
            worker_health_ok: parse_bool(&self.setting_value("workerHealthOk", "false")?),
            worker_error: empty_string_to_none(self.setting_value("workerError", "")?),
            worker_setup_status: enum_from_value(
                &self.setting_value("workerSetupStatus", "missing")?,
            )?,
            worker_setup_step: self.setting_value("workerSetupStep", "")?,
            worker_setup_error: empty_string_to_none(self.setting_value("workerSetupError", "")?),
            cuda_available: parse_bool(&self.setting_value("cudaAvailable", "false")?),
            cuda_error: empty_string_to_none(self.setting_value("cudaError", "")?),
        })
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

    pub(crate) fn recordings(&self) -> rusqlite::Result<Vec<Recording>> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, title, started_at, ended_at, duration_seconds, status, artifact_directory, capture_errors
            FROM recordings
            WHERE status != ?1
            ORDER BY started_at DESC
            ",
        )?;
        let rows = statement
            .query_map(params![enum_value(RecordingStatus::Recording)?], |row| {
                self.row_to_recording(row)
            })?;
        let mut recordings = Vec::new();

        for recording in rows {
            recordings.push(recording?);
        }

        Ok(recordings)
    }

    pub(crate) fn jobs(&self) -> rusqlite::Result<Vec<PipelineJob>> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, recording_id, stage, status, progress, message
            FROM pipeline_jobs
            ORDER BY recording_id DESC, stage
            ",
        )?;
        let rows = statement.query_map([], row_to_pipeline_job)?;
        let mut jobs = Vec::new();

        for job in rows {
            jobs.push(job?);
        }

        Ok(jobs)
    }

    pub(crate) fn recording_by_status(
        &self,
        status: RecordingStatus,
    ) -> rusqlite::Result<Option<Recording>> {
        self.connection
            .query_row(
                "
                SELECT id, title, started_at, ended_at, duration_seconds, status, artifact_directory, capture_errors
                FROM recordings
                WHERE status = ?1
                ORDER BY started_at DESC
                LIMIT 1
                ",
                params![enum_value(status)?],
                |row| self.row_to_recording(row),
            )
            .optional()
    }

    pub(crate) fn recording_by_id(
        &self,
        recording_id: &str,
    ) -> rusqlite::Result<Option<Recording>> {
        self.connection
            .query_row(
                "
                SELECT id, title, started_at, ended_at, duration_seconds, status, artifact_directory, capture_errors
                FROM recordings
                WHERE id = ?1
                ",
                params![recording_id],
                |row| self.row_to_recording(row),
            )
            .optional()
    }

    pub(crate) fn row_to_recording(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<Recording> {
        let id = row.get::<_, String>(0)?;
        let artifact_directory = row.get::<_, String>(6)?;
        let capture_errors = serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default();

        Ok(Recording {
            id: id.clone(),
            title: row.get(1)?,
            started_at: row.get(2)?,
            ended_at: row.get(3)?,
            duration_seconds: row.get(4)?,
            status: enum_from_value(&row.get::<_, String>(5)?)?,
            artifact_directory: artifact_directory.clone(),
            capture_errors,
            stages: self.stages(&id)?,
            artifacts: self.artifacts(&id)?,
            speaker_labels: speaker_labels(&artifact_directory),
        })
    }

    pub(crate) fn stages(&self, recording_id: &str) -> rusqlite::Result<Vec<PipelineStage>> {
        let mut statement = self.connection.prepare(
            "
            SELECT stage, status, progress, message
            FROM pipeline_jobs
            WHERE recording_id = ?1
            ORDER BY CASE stage
                WHEN 'recording' THEN 1
                WHEN 'transcription' THEN 2
                WHEN 'alignment' THEN 3
                WHEN 'diarization' THEN 4
                WHEN 'summary' THEN 5
                ELSE 6
            END
            ",
        )?;
        let rows = statement.query_map(params![recording_id], |row| {
            let id = enum_from_value::<PipelineStageId>(&row.get::<_, String>(0)?)?;

            Ok(PipelineStage {
                id,
                label: stage_label(id).to_owned(),
                status: enum_from_value(&row.get::<_, String>(1)?)?,
                progress: row.get(2)?,
                message: row.get(3)?,
            })
        })?;
        let mut stages = Vec::new();

        for stage in rows {
            stages.push(stage?);
        }

        Ok(stages)
    }

    pub(crate) fn artifacts(&self, recording_id: &str) -> rusqlite::Result<Vec<Artifact>> {
        let mut statement = self.connection.prepare(
            "
            SELECT kind, label, path, ready
            FROM recording_artifacts
            WHERE recording_id = ?1
            ORDER BY kind
            ",
        )?;
        let rows = statement.query_map(params![recording_id], |row| {
            Ok(Artifact {
                kind: enum_from_value(&row.get::<_, String>(0)?)?,
                label: row.get(1)?,
                path: row.get(2)?,
                ready: row.get::<_, bool>(3)?,
            })
        })?;
        let mut artifacts = Vec::new();

        for artifact in rows {
            artifacts.push(artifact?);
        }

        Ok(artifacts)
    }
}

fn speaker_labels(artifact_directory: &str) -> Vec<SpeakerLabel> {
    let path = PathBuf::from(artifact_directory).join("diarization.json");
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };
    let Some(turns) = value.get("turns").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    let mut labels = Vec::new();

    for turn in turns {
        let Some(name) = turn.get("speaker").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if labels.iter().any(|label: &SpeakerLabel| label.name == name) {
            continue;
        }
        labels.push(SpeakerLabel {
            name: name.to_owned(),
        });
    }

    labels
}

fn secret_configured(value: &str) -> bool {
    !value.trim().is_empty()
}
