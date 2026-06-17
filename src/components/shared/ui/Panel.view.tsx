import { clsx } from 'clsx';
import type { ComponentChildren, JSX } from 'preact';

interface PanelProps {
  as?: 'article' | 'div' | 'section';
  children: ComponentChildren;
  class?: string;
  gap?: 'none' | 'sm' | 'md' | 'lg';
  padding?: 'sm' | 'md' | 'lg';
  surface?: 'card' | 'input';
}

const GAP_CLASS: Record<NonNullable<PanelProps['gap']>, string> = {
  none: 'gap-0',
  sm: 'gap-3',
  md: 'gap-4',
  lg: 'gap-5'
};

const PADDING_CLASS: Record<NonNullable<PanelProps['padding']>, string> = {
  sm: 'p-3',
  md: 'p-4',
  lg: 'p-5'
};

const SURFACE_CLASS: Record<NonNullable<PanelProps['surface']>, string> = {
  card: 'bg-bg-card',
  input: 'bg-bg-input'
};

export const Panel = ({
  as = 'article',
  children,
  class: classProp,
  gap = 'md',
  padding = 'lg',
  surface = 'card'
}: PanelProps): JSX.Element => {
  const panelClass = clsx(
    'flex min-w-0 flex-col border border-border-base',
    SURFACE_CLASS[surface],
    PADDING_CLASS[padding],
    GAP_CLASS[gap],
    classProp
  );

  if (as === 'section') {
    return <section class={panelClass}>{children}</section>;
  }

  if (as === 'div') {
    return <div class={panelClass}>{children}</div>;
  }

  return <article class={panelClass}>{children}</article>;
};
