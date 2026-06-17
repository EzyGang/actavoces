import { Button as BaseButton } from '@base-ui/react/button';
import { clsx } from 'clsx';
import type { ComponentChildren, JSX, MouseEventHandler } from 'preact';

interface ButtonProps {
  children: ComponentChildren;
  'aria-label'?: string;
  disabled?: boolean;
  variant?: 'primary' | 'secondary' | 'ghost';
  onClick?: MouseEventHandler<HTMLButtonElement>;
  title?: string;
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
  'aria-label': ariaLabel,
  disabled = false,
  variant = 'secondary',
  onClick,
  title,
  type = 'button',
  class: classProp
}: ButtonProps): JSX.Element => (
  <BaseButton
    aria-label={ariaLabel}
    className={clsx(
      'inline-flex h-11 items-center justify-center gap-2 px-4 font-semibold text-xs uppercase tracking-wider transition duration-fast focus-visible:ring-2 focus-visible:ring-border-focus focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-40',
      VARIANT_CLASS[variant],
      classProp
    )}
    disabled={disabled}
    onClick={onClick}
    title={title}
    type={type}
  >
    {children}
  </BaseButton>
);
