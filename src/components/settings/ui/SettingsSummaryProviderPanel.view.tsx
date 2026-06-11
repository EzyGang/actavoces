import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';

interface SettingsPanelProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsSummaryProviderPanel = ({ app }: SettingsPanelProps): JSX.Element => (
  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
    <h2 class='font-semibold text-xl'>Summary provider</h2>
    <label class='flex items-center gap-3 text-sm'>
      <input
        checked={app.settings.toggles.summaryEnabled.checked}
        class='h-4 w-4'
        onInput={app.settings.toggles.summaryEnabled.onInput}
        type='checkbox'
      />
      <span>Enable summary generation</span>
    </label>
    <div class='grid gap-3'>
      {app.settings.textFields.map((field) => (
        <label class='flex flex-col gap-2 text-sm' key={field.key}>
          <span class='text-text-muted'>{field.label}</span>
          <input
            class='h-11 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
            onInput={field.onInput}
            type={field.inputType ?? 'text'}
            value={field.value}
          />
        </label>
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
  </article>
);
