import { Input as BaseInput } from '@base-ui/react/input';
import { clsx } from 'clsx';
import type { JSX, KeyboardEventHandler } from 'preact';

interface InputProps {
  'aria-label'?: string;
  autofocus?: boolean;
  class?: string;
  disabled?: boolean;
  invalid?: boolean;
  min?: string;
  onInput?: JSX.InputEventHandler<HTMLInputElement>;
  onKeyDown?: KeyboardEventHandler<HTMLInputElement>;
  placeholder?: string;
  readOnly?: boolean;
  surface?: 'input' | 'card';
  type?: 'number' | 'password' | 'text';
  value?: number | string;
}

export const Input = ({
  'aria-label': ariaLabel,
  autofocus = false,
  class: classProp,
  disabled = false,
  invalid = false,
  min,
  onInput,
  onKeyDown,
  placeholder,
  readOnly = false,
  surface = 'input',
  type = 'text',
  value
}: InputProps): JSX.Element => (
  <BaseInput
    aria-label={ariaLabel}
    autoFocus={autofocus}
    className={clsx(
      'h-11 border px-3 font-mono text-xs text-text-primary outline-none transition-colors duration-fast placeholder:text-text-muted focus:border-border-focus focus-visible:ring-2 focus-visible:ring-border-focus disabled:cursor-not-allowed disabled:opacity-50',
      surface === 'card' ? 'bg-bg-card' : 'bg-bg-input',
      invalid ? 'border-error-border' : 'border-border-base',
      classProp
    )}
    disabled={disabled}
    min={min}
    onInput={onInput}
    onKeyDown={onKeyDown}
    placeholder={placeholder}
    readOnly={readOnly}
    type={type}
    value={value}
  />
);
