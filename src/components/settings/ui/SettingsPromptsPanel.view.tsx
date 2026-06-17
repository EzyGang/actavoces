import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Field } from '../../shared/ui/Field.view';
import { Panel } from '../../shared/ui/Panel.view';
import { Textarea } from '../../shared/ui/Textarea.view';

interface SettingsPanelProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsPromptsPanel = ({ app }: SettingsPanelProps): JSX.Element => (
  <Panel>
    <h2 class='font-semibold text-xl'>Prompts</h2>
    {app.settings.textareaFields.map((field) => (
      <Field key={field.key}>
        <Field.Label>{field.label}</Field.Label>
        <Textarea class='min-h-28 resize-y' onInput={field.onInput} value={field.value} />
      </Field>
    ))}
  </Panel>
);
