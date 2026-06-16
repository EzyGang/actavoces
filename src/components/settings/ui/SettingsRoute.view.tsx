import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';
import { SettingsGeneralCapturePanel } from './SettingsGeneralCapturePanel.view';
import { SettingsPromptsPanel } from './SettingsPromptsPanel.view';
import { SettingsSummaryProviderPanel } from './SettingsSummaryProviderPanel.view';
import { SettingsTranscriptionSpeakersPanel } from './SettingsTranscriptionSpeakersPanel.view';

interface SettingsRouteProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsRoute = ({ app }: SettingsRouteProps): JSX.Element => (
  <section class='flex flex-col gap-5'>
    <div class='flex items-center justify-between gap-4'>
      <div class='flex flex-col gap-1'>
        <h1 class='font-semibold text-2xl'>Settings</h1>
        <p class='text-sm text-text-muted'>
          Paths, capture defaults, worker setup, speakers, and provider settings.
        </p>
      </div>
      <Button
        disabled={
          !app.status.hasUnsavedSettings.value ||
          app.status.savingSettings.value ||
          app.data.settingsValidationErrors.value.length > 0
        }
        onClick={app.actions.saveSettings}
        variant='primary'
      >
        Save settings
      </Button>
    </div>

    {app.data.settingsValidationErrors.value.length > 0 ? (
      <div class='flex flex-col gap-2 border border-warning-border bg-warning-bg p-4 text-sm text-warning'>
        {app.data.settingsValidationErrors.value.map((error) => (
          <span key={error}>{error}</span>
        ))}
      </div>
    ) : null}

    <section class='grid gap-4 xl:grid-cols-2'>
      <SettingsGeneralCapturePanel app={app} />
      <SettingsTranscriptionSpeakersPanel app={app} />
    </section>

    <section class='grid gap-4 xl:grid-cols-2'>
      <SettingsSummaryProviderPanel app={app} />
      <SettingsPromptsPanel app={app} />
    </section>
  </section>
);
