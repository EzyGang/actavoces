use rusqlite::params;

use crate::domain::types::*;
use crate::storage::repository::AppRepository;
use crate::utils::{empty_string_to_none, enum_from_value, enum_value, parse_bool};

impl AppRepository {
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
            WorkerSetupStatus::Failed => {
                desktop_status.worker_error = progress.error.clone();
            }
            WorkerSetupStatus::Missing
            | WorkerSetupStatus::Installing
            | WorkerSetupStatus::Ready => (),
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
}
