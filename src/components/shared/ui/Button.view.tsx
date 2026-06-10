import { clsx } from 'clsx';
import type { ComponentChildren, JSX, MouseEventHandler } from 'preact';

interface ButtonProps {
  children: ComponentChildren;
  disabled?: boolean;
  variant?: 'primary' | 'secondary' | 'ghost';
  onClick?: MouseEventHandler<HTMLButtonElement>;
  type?: 'button' | 'submit';
  class?: string;
}

const VARIANT_CLASS: Record<NonNullable<ButtonProps['variant']>, string> = {
  primary: 'bg-text-primary text-bg-page hover:opacity-90',
  secondary:
    'border border-border-base bg-transparent text-text-primary hover:border-text-muted hover:bg-bg-hover',
  ghost: 'text-text-secondary hover:bg-bg-hover hover:text-text-primary'
};

export const Button = ({
  children,
  disabled = false,
  variant = 'secondary',
  onClick,
  type = 'button',
  class: classProp
}: ButtonProps): JSX.Element => (
  <button
    class={clsx(
      'inline-flex h-11 items-center justify-center gap-2 px-4 font-semibold text-xs uppercase tracking-wider transition duration-fast disabled:cursor-not-allowed disabled:opacity-40',
      VARIANT_CLASS[variant],
      classProp
    )}
    disabled={disabled}
    onClick={onClick}
    type={type}
  >
    {children}
  </button>
);
