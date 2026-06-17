import type { JSX } from 'preact';
import { Button } from '../../shared/ui/Button.view';
import { Field } from '../../shared/ui/Field.view';
import { Input } from '../../shared/ui/Input.view';
import { Panel } from '../../shared/ui/Panel.view';
import type { DashboardViewModel } from '../hooks/useDashboard.hook';

interface DashboardGlossaryPanelProps {
  field: DashboardViewModel['data']['glossaryField'];
  saving: boolean;
}

export const DashboardGlossaryPanel = ({
  field,
  saving
}: DashboardGlossaryPanelProps): JSX.Element => (
  <Panel>
    <div class='flex items-center justify-between gap-4'>
      <h2 class='font-semibold text-xl'>Glossary</h2>
      <span class='font-semibold text-3xl'>{field.entries.length}</span>
    </div>
    <div class='flex flex-col gap-3 text-sm'>
      <Field class='text-sm'>
        <Field.Label>Transcription hints</Field.Label>
        <div class='flex gap-2'>
          <Input
            class='min-w-0 flex-1'
            disabled={saving}
            onInput={field.onInput}
            onKeyDown={field.onKeyDown}
            placeholder={field.placeholder}
            type='text'
            value={field.value}
          />
          <Button class='h-11 px-3' disabled={saving} onClick={field.onAdd} variant='secondary'>
            Add
          </Button>
        </div>
        <Field.Description>{field.hint}</Field.Description>
      </Field>
      {field.entries.length > 0 ? (
        <div class='flex flex-wrap gap-2'>
          {field.entries.map((entry) => (
            <span
              class='inline-flex items-center gap-2 border border-border-base bg-bg-input px-2 py-1 font-mono text-xs'
              key={entry.value}
            >
              {entry.value}
              <Button
                aria-label={`Remove ${entry.value}`}
                class='h-auto p-0! text-text-muted hover:text-text-primary'
                disabled={saving}
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
  </Panel>
);
