import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { useApp } from '../hooks/useApp.hook';

interface RecordingOverlayViewProps {
  app: ReturnType<typeof useApp>;
}

export const RecordingOverlayView = ({ app }: RecordingOverlayViewProps): JSX.Element => (
  <main class='min-h-screen bg-bg-page text-text-primary'>
    <section class='flex h-screen items-center justify-between gap-3 border border-error-border bg-bg-card p-3'>
      <div class='flex min-w-0 flex-col gap-1'>
        <StatusBadge
          label={app.status.isRecording.value ? 'Recording' : 'Stopping'}
          status={app.status.isRecording.value ? 'recording' : 'pending'}
        />
        <span class='truncate font-mono text-text-muted text-[11px] uppercase tracking-[0.05em]'>
          ActaVoces
        </span>
      </div>
      <Button
        class='h-10 px-3'
        disabled={app.status.loading.value || !app.status.isRecording.value}
        onClick={app.actions.stopRecording}
        variant='secondary'
      >
        Stop
      </Button>
    </section>
  </main>
);
