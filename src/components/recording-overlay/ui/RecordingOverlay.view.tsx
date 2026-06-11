import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { useRecordingOverlay } from '../hooks/useRecordingOverlay.hook';

interface RecordingOverlayViewProps {
  overlay: ReturnType<typeof useRecordingOverlay>;
}

export const RecordingOverlayView = ({ overlay }: RecordingOverlayViewProps): JSX.Element => (
  <main class='min-h-screen bg-bg-page text-text-primary'>
    {overlay.status.displayMode.value === 'minimal' ? (
      <div class='min-h-screen flex items-center justify-center h-full p-3'>
        <StatusBadge
          label={overlay.status.stopping.value ? 'S' : 'R'}
          status={overlay.status.stopping.value ? 'pending' : 'recording'}
        />
      </div>
    ) : (
      <section class='grid h-screen grid-cols-[minmax(0,1fr)_auto] items-center gap-4 overflow-hidden border border-error-border bg-bg-card px-4'>
        <div class='flex min-w-0 items-center gap-3'>
          <span class='h-3 w-3 shrink-0 rounded-full bg-error' />
          <div class='flex min-w-0 flex-col gap-1.5'>
            <span class='truncate font-semibold text-sm leading-none'>
              {overlay.status.stopping.value ? 'Stopping' : 'Recording'}
            </span>
            <span class='truncate font-mono text-[11px] text-text-muted uppercase leading-none'>
              ActaVoces
            </span>
          </div>
        </div>
        <Button
          class='h-9 shrink-0 px-4 text-[11px] leading-none'
          disabled={overlay.status.stopping.value}
          onClick={overlay.actions.stopRecording}
          variant='secondary'
        >
          Stop
        </Button>
      </section>
    )}
  </main>
);
