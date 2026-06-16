mod jobs;
mod queries;
mod recordings;
mod runtime;
mod settings;

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::domain::types::*;
use crate::settings::{
    default_model_inventory, default_settings, HUGGING_FACE_TOKEN_SETTING,
    SUMMARY_PROVIDER_API_KEY_SETTING,
};
use crate::utils::{enum_value, json_string, option_number_to_string};
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
        )?;
        self.remove_alignment_stage()
    }

    fn remove_alignment_stage(&self) -> rusqlite::Result<()> {
        self.connection
            .execute("DELETE FROM pipeline_jobs WHERE stage = 'alignment'", [])?;
        self.connection
            .execute("DELETE FROM job_events WHERE stage = 'alignment'", [])?;

        Ok(())
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
}
