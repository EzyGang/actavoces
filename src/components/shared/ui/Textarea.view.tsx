import { clsx } from 'clsx';
import type { JSX } from 'preact';
import { forwardRef } from 'preact/compat';

interface TextareaProps extends JSX.TextareaHTMLAttributes<HTMLTextAreaElement> {
  invalid?: boolean;
  surface?: 'input' | 'card';
}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ class: classProp, invalid = false, surface = 'input', ...rest }, ref): JSX.Element => (
    <textarea
      class={clsx(
        'border p-3 font-mono text-xs text-text-primary outline-none transition-colors duration-fast placeholder:text-text-muted focus:border-border-focus focus-visible:ring-2 focus-visible:ring-border-focus disabled:cursor-not-allowed disabled:opacity-50',
        surface === 'card' ? 'bg-bg-card' : 'bg-bg-input',
        invalid ? 'border-error-border' : 'border-border-base',
        classProp
      )}
      ref={ref}
      {...rest}
    />
  )
);
