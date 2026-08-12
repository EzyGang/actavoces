import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { Field } from '../../shared/ui/Field.view';
import { Input } from '../../shared/ui/Input.view';
import type { SettingsGlossaryField as SettingsGlossaryFieldData } from '../hooks/settings.helpers';

interface SettingsGlossaryFieldProps {
  field: SettingsGlossaryFieldData;
}

export const SettingsGlossaryField = ({ field }: SettingsGlossaryFieldProps): JSX.Element => (
  <div class='flex flex-col gap-3 border border-border-base bg-bg-input p-3 text-sm'>
    <Field class='text-sm'>
      <Field.Label>{field.label}</Field.Label>
      <div class='flex gap-2'>
        <Input
          class='min-w-0 flex-1'
          onInput={field.onInput}
          onKeyDown={field.onKeyDown}
          placeholder={field.placeholder}
          type='text'
          value={field.value}
          surface='card'
        />
        <Button class='h-11 px-3' onClick={field.onAdd} variant='secondary'>
          Add
        </Button>
      </div>
      <Field.Description>{field.hint}</Field.Description>
    </Field>
    {field.entries.length > 0 ? (
      <div class='flex flex-wrap gap-2'>
        {field.entries.map((entry) => (
          <span
            class='inline-flex items-center gap-2 border border-border-base bg-bg-card px-2 py-1 font-mono text-xs'
            key={entry.value}
          >
            {entry.value}
            <Button
              aria-label={`Remove ${entry.value}`}
              class='h-auto p-0! text-text-muted hover:text-text-primary'
              onClick={entry.onRemove}
              variant='ghost'
            >
              X
            </Button>
          </span>
        ))}
      </div>
    ) : null}
  </div>
);
