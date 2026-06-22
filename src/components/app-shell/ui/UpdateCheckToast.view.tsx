import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';

interface UpdateCheckToastProps {
  message: string;
  onDismiss: () => void;
}

export const UpdateCheckToast = ({ message, onDismiss }: UpdateCheckToastProps): JSX.Element => (
  <aside class='fixed top-4 right-4 z-50 flex w-[min(360px,calc(100vw-32px))] flex-col gap-3 border border-success-border bg-bg-page p-4 text-success'>
    <span class='font-mono text-[11px] uppercase tracking-wider'>Update check</span>
    <div class='flex items-center justify-between gap-3'>
      <p class='text-sm'>{message}</p>
      <Button class='h-9 px-3' onClick={onDismiss} variant='ghost'>
        Dismiss
      </Button>
    </div>
  </aside>
);
