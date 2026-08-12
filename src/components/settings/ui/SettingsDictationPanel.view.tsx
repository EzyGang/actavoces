import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';
import { Field } from '../../shared/ui/Field.view';
import { Panel } from '../../shared/ui/Panel.view';
import { Select } from '../../shared/ui/Select.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import { SettingsGlossaryField } from './SettingsGlossaryField.view';

interface SettingsDictationPanelProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsDictationPanel = ({ app }: SettingsDictationPanelProps): JSX.Element => (
  <Panel>
    <h2 class='font-semibold text-xl'>Dictation</h2>
    <p class='text-sm text-text-muted'>Uses the recording microphone and compute configuration.</p>
    <Field>
      <Field.Label>{app.settings.dictationHotkeyField.label}</Field.Label>
      <Button
        class='justify-start font-mono normal-case'
        onClick={app.settings.dictationHotkeyField.onCapture}
        variant='secondary'
      >
        {app.settings.dictationHotkeyField.recording
          ? 'Press shortcut'
          : app.settings.dictationHotkeyField.displayValue}
      </Button>
      <Field.Description>
        Single ordinary keys and modifier combinations are supported. Escape cancels capture.
      </Field.Description>
    </Field>
    <div class='grid gap-3 md:grid-cols-2'>
      {app.settings.dictationSelectFields.map((field) => (
        <Field key={field.key}>
          <Field.Label>{field.label}</Field.Label>
          <Select onValueChange={field.onValueChange} options={field.options} value={field.value} />
        </Field>
      ))}
    </div>
    <SettingsGlossaryField field={app.settings.dictationHintsField} />
    <div class='flex flex-col gap-3 border border-border-base bg-bg-input p-3 text-sm'>
      <div class='flex items-center justify-between gap-3'>
        <div class='flex flex-col gap-1'>
          <span class='text-text-muted'>Selected model</span>
          <span class='font-mono text-xs'>{app.settings.draft.value.dictationWhisperModel}</span>
        </div>
        <StatusBadge
          label={app.data.selectedDictationModel.value?.installed ? 'Installed' : 'Missing'}
          status={app.data.selectedDictationModel.value?.installed ? 'complete' : 'pending'}
        />
      </div>
      <div class='flex flex-wrap gap-2'>
        <Button class='h-9 px-3' onClick={app.actions.refreshModels} variant='ghost'>
          Refresh inventory
        </Button>
        <Button
          class='h-9 px-3'
          disabled={
            app.status.installingModel.value || app.data.selectedDictationModel.value?.installed
          }
          onClick={app.actions.installSelectedDictationModel}
          variant='secondary'
        >
          Install selected
        </Button>
      </div>
    </div>
  </Panel>
);
