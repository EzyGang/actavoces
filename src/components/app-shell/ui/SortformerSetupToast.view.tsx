import type { JSX } from 'preact';
import type { SortformerSetupProgress } from '../../../types/desktop';

interface SortformerSetupToastProps {
  progress: SortformerSetupProgress;
}

export const SortformerSetupToast = ({ progress }: SortformerSetupToastProps): JSX.Element => (
  <aside
    class={
      progress.status === 'failed'
        ? 'fixed bottom-4 left-4 z-50 flex w-[min(360px,calc(100vw-32px))] flex-col gap-3 border border-error-border bg-bg-page p-4 text-error'
        : progress.status === 'ready'
          ? 'fixed bottom-4 left-4 z-50 flex w-[min(360px,calc(100vw-32px))] flex-col gap-3 border border-success-border bg-bg-page p-4 text-success'
          : 'fixed bottom-4 left-4 z-50 flex w-[min(360px,calc(100vw-32px))] flex-col gap-3 border border-border-base bg-bg-page p-4 text-text-primary'
    }
  >
    <div class='flex items-center justify-between gap-4'>
      <span class='font-mono text-[11px] uppercase tracking-wider'>Voice attribution setup</span>
      {progress.progress === null ? null : (
        <span class='font-mono text-[11px]'>{progress.progress}%</span>
      )}
    </div>
    <div class='flex flex-col gap-2'>
      <p class='text-sm'>{progress.error ?? progress.step}</p>
      {progress.progress === null ? null : (
        <div class='h-1.5 border border-border-base bg-bg-card'>
          <div
            class={
              progress.status === 'failed'
                ? 'h-full bg-error'
                : progress.status === 'ready'
                  ? 'h-full bg-success'
                  : 'h-full bg-text-primary'
            }
            style={{ width: `${progress.progress}%` }}
          />
        </div>
      )}
    </div>
  </aside>
);
