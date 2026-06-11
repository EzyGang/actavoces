use rusqlite::params;

use crate::domain::types::*;
use crate::storage::repository::AppRepository;
use crate::utils::{enum_value, row_to_pipeline_job, unix_timestamp};

impl AppRepository {
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
}
