use std::fs;
use std::path::PathBuf;

use rusqlite::{params, OptionalExtension};

use crate::artifacts::{diarization_path, stage_label};
use crate::capture::audio::capture_devices;
use crate::domain::types::*;
use crate::storage::repository::AppRepository;
use crate::utils::{enum_from_value, enum_value, row_to_pipeline_job};

impl AppRepository {
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
                WHEN 'diarization' THEN 3
                WHEN 'summary' THEN 4
                ELSE 5
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
    let path = diarization_path(&PathBuf::from(artifact_directory));
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
