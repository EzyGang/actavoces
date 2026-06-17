import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';
import { Field } from '../../shared/ui/Field.view';
import { Input } from '../../shared/ui/Input.view';
import { Panel } from '../../shared/ui/Panel.view';
import { Switch } from '../../shared/ui/Switch.view';

interface SettingsPanelProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsSummaryProviderPanel = ({ app }: SettingsPanelProps): JSX.Element => (
  <Panel>
    <h2 class='font-semibold text-xl'>Summary provider</h2>
    <Switch
      checked={app.settings.toggles.summaryEnabled.checked}
      onCheckedChange={app.settings.toggles.summaryEnabled.onCheckedChange}
    >
      Enable summary generation
    </Switch>
    <div class='grid gap-3'>
      {app.settings.textFields.map((field) => (
        <Field key={field.key}>
          <Field.Label>{field.label}</Field.Label>
          <Input onInput={field.onInput} type={field.inputType ?? 'text'} value={field.value} />
        </Field>
      ))}
    </div>
    <div class='flex items-center justify-between gap-3 border border-border-base bg-bg-input p-3 text-sm'>
      <div class='flex flex-col gap-1'>
        <span class='text-text-muted'>API key status</span>
        <span>
          {app.data.snapshot.value.settings.providerApiKeyConfigured
            ? 'Saved in local database'
            : 'Missing'}
        </span>
      </div>
      <Button
        class='h-9 px-3'
        disabled={
          app.status.savingSettings.value ||
          !app.data.snapshot.value.settings.providerApiKeyConfigured
        }
        onClick={app.actions.clearProviderApiKey}
        variant='ghost'
      >
        Clear key
      </Button>
    </div>
  </Panel>
);
