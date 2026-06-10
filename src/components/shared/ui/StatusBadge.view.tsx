import { clsx } from 'clsx';
import type { JSX } from 'preact';
import type { PipelineStageStatus, RecordingStatus } from '../../../types/desktop';

type BadgeStatus = PipelineStageStatus | RecordingStatus;

interface StatusBadgeProps {
  label: string;
  status: BadgeStatus;
}

const STATUS_CLASS: Record<BadgeStatus, string> = {
  complete: 'border-success-border bg-success-bg text-success',
  failed: 'border-error-border bg-error-bg text-error',
  idle: 'border-border-base bg-bg-input text-text-muted',
  needsSetup: 'border-warning-border bg-warning-bg text-warning',
  pending: 'border-border-base bg-bg-input text-text-muted',
  processing: 'border-warning-border bg-warning-bg text-warning',
  recording: 'border-error-border bg-error-bg text-error',
  running: 'border-accent bg-bg-input text-accent-light',
  skipped: 'border-warning-border bg-warning-bg text-warning'
};

const STATUS_LABEL: Partial<Record<BadgeStatus, string>> = {
  needsSetup: 'Needs setup'
};

export const StatusBadge = ({ label, status }: StatusBadgeProps): JSX.Element => (
  <span
    class={clsx(
      'inline-flex h-7 items-center border px-2.5 font-mono text-[11px] uppercase tracking-wider',
      STATUS_CLASS[status]
    )}
  >
    {STATUS_LABEL[status] ?? label}
  </span>
);
