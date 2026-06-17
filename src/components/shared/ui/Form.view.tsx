import { Form as BaseForm } from '@base-ui/react/form';
import { clsx } from 'clsx';
import type { ComponentChildren, JSX } from 'preact';

interface FormProps {
  children: ComponentChildren;
  class?: string;
  onSubmit?: JSX.GenericEventHandler<HTMLFormElement>;
}

export const Form = ({ children, class: classProp, onSubmit }: FormProps): JSX.Element => (
  <BaseForm className={clsx(classProp)} onSubmit={onSubmit}>
    {children}
  </BaseForm>
);
