import type { PipelineStage, Recording } from '../../../types/desktop';

export const resolveLatestRecording = (recordings: Recording[]): Recording | null =>
  recordings.length > 0 ? recordings[0] : null;

export const canRetryRecording = (recording: Recording): boolean =>
  recording.stages.some(
    (stage) =>
      stage.id !== 'recording' && (stage.status === 'failed' || stage.status === 'needsSetup')
  );

export const canRerunSummary = (recording: Recording): boolean => {
  const transcription = recording.stages.find((stage) => stage.id === 'transcription');
  const diarization = recording.stages.find((stage) => stage.id === 'diarization');
  const summary = recording.stages.find((stage) => stage.id === 'summary');

  if (!transcription || !diarization || !summary) {
    return false;
  }

  return (
    transcription.status === 'complete' &&
    (diarization.status === 'complete' || diarization.status === 'skipped') &&
    summary.status !== 'pending' &&
    summary.status !== 'running'
  );
};

const stageProgressWeight = (stage: PipelineStage): number => {
  if (stage.status === 'complete' || stage.status === 'skipped') {
    return 100;
  }

  return stage.progress;
};

export const recordingProgress = (recording: Recording): number => {
  if (recording.stages.length === 0) {
    return 0;
  }

  const progress = recording.stages.reduce((total, stage) => total + stageProgressWeight(stage), 0);

  return Math.round(progress / recording.stages.length);
};

export const recordingPipelineStatus = (recording: Recording) => {
  const blocker = recording.stages.find(
    (stage) => stage.status === 'failed' || stage.status === 'needsSetup'
  );

  if (blocker) {
    return {
      status: blocker.status,
      label: blocker.status,
      message: blocker.message
    };
  }

  if (recording.stages.some((stage) => stage.status === 'running')) {
    return {
      status: 'running' as const,
      label: 'running',
      message: 'Processing recording'
    };
  }

  if (
    recording.stages.every((stage) => stage.status === 'complete' || stage.status === 'skipped')
  ) {
    return {
      status: 'complete' as const,
      label: 'complete',
      message: 'Processing complete'
    };
  }

  return {
    status: 'pending' as const,
    label: 'pending',
    message: 'Waiting for processing'
  };
};
