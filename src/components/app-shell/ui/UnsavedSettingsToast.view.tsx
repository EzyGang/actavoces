import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';

interface UnsavedSettingsToastProps {
  offset: boolean;
  saving: boolean;
  onSave: () => void;
}

export const UnsavedSettingsToast = ({
  offset,
  saving,
  onSave
}: UnsavedSettingsToastProps): JSX.Element => (
  <aside
    class={
      offset
        ? 'fixed bottom-36 left-4 z-50 flex w-[min(360px,calc(100vw-32px))] flex-col gap-3 border border-warning-border bg-bg-page p-4 text-warning'
        : 'fixed bottom-4 left-4 z-50 flex w-[min(360px,calc(100vw-32px))] flex-col gap-3 border border-warning-border bg-bg-page p-4 text-warning'
    }
  >
    <span class='font-mono text-[11px] uppercase tracking-wider'>Unsaved settings</span>
    <div class='flex items-center justify-between gap-3'>
      <p class='text-sm'>Settings have changes that are not saved yet.</p>
      <Button class='h-9 px-3' disabled={saving} onClick={onSave} variant='ghost'>
        Save
      </Button>
    </div>
  </aside>
);
