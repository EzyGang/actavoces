import { Field as BaseField } from '@base-ui/react/field';
import { clsx } from 'clsx';
import type { ComponentChildren, JSX } from 'preact';

interface FieldProps {
  children: ComponentChildren;
  class?: string;
}

interface FieldLabelProps {
  children: ComponentChildren;
  class?: string;
}

interface FieldDescriptionProps {
  children: ComponentChildren;
  class?: string;
}

export const Field = ({ children, class: classProp }: FieldProps): JSX.Element => (
  <BaseField.Root className={clsx('flex flex-col gap-2 text-sm', classProp)}>
    {children}
  </BaseField.Root>
);

Field.Label = ({ children, class: classProp }: FieldLabelProps): JSX.Element => (
  <BaseField.Label className={clsx('text-text-muted', classProp)}>{children}</BaseField.Label>
);

Field.Description = ({ children, class: classProp }: FieldDescriptionProps): JSX.Element => (
  <BaseField.Description className={clsx('text-text-muted text-xs', classProp)}>
    {children}
  </BaseField.Description>
);
