use std::path::{Path, PathBuf};

use rusqlite::params;

use crate::artifacts::{recording_stages, stage_message};
use crate::domain::types::*;
use crate::storage::repository::{AppRepository, NewRecording};
use crate::utils::{enum_value, json_string, remove_artifact_directory, unix_timestamp};

impl AppRepository {
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
                profile,
                artifact_directory,
                capture_errors
            )
            VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?5, ?6, '[]')
            ",
            params![
                recording.id,
                recording.title,
                recording.started_at,
                enum_value(RecordingStatus::Recording)?,
                enum_value(recording.profile)?,
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

    pub(crate) fn update_recording_title_and_artifact_directory(
        &mut self,
        recording_id: &str,
        title: &str,
        current_directory: &Path,
        artifact_directory: &Path,
    ) -> rusqlite::Result<()> {
        let artifacts = self.artifacts(recording_id)?;
        let transaction = self.connection.transaction()?;

        transaction.execute(
            "
            UPDATE recordings
            SET title = ?1,
                artifact_directory = ?2
            WHERE id = ?3
            ",
            params![
                title,
                artifact_directory.display().to_string(),
                recording_id
            ],
        )?;

        for artifact in artifacts {
            transaction.execute(
                "
                UPDATE recording_artifacts
                SET path = ?1
                WHERE recording_id = ?2
                    AND kind = ?3
                ",
                params![
                    renamed_artifact_path(&artifact.path, current_directory, artifact_directory),
                    recording_id,
                    enum_value(artifact.kind)?,
                ],
            )?;
        }

        transaction.commit()
    }
}

fn renamed_artifact_path(
    path: &str,
    current_directory: &Path,
    artifact_directory: &Path,
) -> String {
    let path = PathBuf::from(path);

    match path.strip_prefix(current_directory) {
        Ok(relative_path) => artifact_directory.join(relative_path).display().to_string(),
        Err(_) => path.display().to_string(),
    }
}
