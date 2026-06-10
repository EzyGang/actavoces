import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { useRecordingOverlay } from '../hooks/useRecordingOverlay.hook';

interface RecordingOverlayViewProps {
  overlay: ReturnType<typeof useRecordingOverlay>;
}

export const RecordingOverlayView = ({ overlay }: RecordingOverlayViewProps): JSX.Element => (
  <main class='min-h-screen bg-bg-page text-text-primary'>
    <section class='flex h-screen items-center justify-between gap-3 border border-error-border bg-bg-card p-3'>
      <div class='flex min-w-0 flex-col gap-1'>
        <StatusBadge
          label={overlay.status.stopping.value ? 'Stopping' : 'Recording'}
          status={overlay.status.stopping.value ? 'pending' : 'recording'}
        />
        <span class='truncate font-mono text-text-muted text-[11px] uppercase tracking-wider'>
          ActaVoces
        </span>
      </div>
      <Button
        class='h-10 px-3'
        disabled={overlay.status.stopping.value}
        onClick={overlay.actions.stopRecording}
        variant='secondary'
      >
        Stop
      </Button>
    </section>
  </main>
);
