import { Collapsible as BaseCollapsible } from '@base-ui/react/collapsible';
import { IconChevronRight } from '@tabler/icons-react';
import { clsx } from 'clsx';
import type { ComponentChildren, JSX } from 'preact';

interface CollapsibleProps {
  children: ComponentChildren;
  class?: string;
}

interface CollapsibleTriggerProps {
  children: ComponentChildren;
  class?: string;
}

interface CollapsiblePanelProps {
  children: ComponentChildren;
  class?: string;
}

export const Collapsible = ({ children, class: classProp }: CollapsibleProps): JSX.Element => (
  <BaseCollapsible.Root
    className={clsx('border border-border-base bg-bg-input p-3', classProp)}
    defaultOpen={false}
  >
    {children}
  </BaseCollapsible.Root>
);

Collapsible.Trigger = ({ children, class: classProp }: CollapsibleTriggerProps): JSX.Element => (
  <BaseCollapsible.Trigger
    className={clsx(
      'group flex w-full cursor-pointer items-center justify-between gap-3 text-text-secondary text-sm outline-none focus-visible:ring-2 focus-visible:ring-border-focus',
      classProp
    )}
  >
    <span>{children}</span>
    <IconChevronRight
      aria-hidden='true'
      className='h-4 w-4 transition-transform duration-fast group-data-panel-open:rotate-90'
    />
  </BaseCollapsible.Trigger>
);

Collapsible.Panel = ({ children, class: classProp }: CollapsiblePanelProps): JSX.Element => (
  <BaseCollapsible.Panel className={clsx('pt-3', classProp)}>{children}</BaseCollapsible.Panel>
);
