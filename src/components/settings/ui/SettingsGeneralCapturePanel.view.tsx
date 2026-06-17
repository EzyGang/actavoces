import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';
import { Field } from '../../shared/ui/Field.view';
import { Input } from '../../shared/ui/Input.view';
import { Panel } from '../../shared/ui/Panel.view';
import { Select } from '../../shared/ui/Select.view';
import { Switch } from '../../shared/ui/Switch.view';

interface SettingsPanelProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsGeneralCapturePanel = ({ app }: SettingsPanelProps): JSX.Element => (
  <Panel>
    <h2 class='font-semibold text-xl'>General and capture</h2>
    <div class='grid gap-3 md:grid-cols-2'>
      {app.settings.folderFields.map((field) => (
        <Field key={field.key}>
          <Field.Label>{field.label}</Field.Label>
          <div class='flex gap-2'>
            <Input class='min-w-0 flex-1' readOnly value={field.value} />
            <Button class='h-11 px-3' onClick={field.onSelect} type='button' variant='secondary'>
              Choose
            </Button>
          </div>
        </Field>
      ))}
      <Field>
        <Field.Label>{app.settings.hotkeyField.label}</Field.Label>
        <Button
          class='h-11 justify-start bg-bg-input px-3 text-left font-mono font-normal text-xs normal-case tracking-normal hover:border-border-focus focus:border-border-focus'
          onClick={app.settings.hotkeyField.onCapture}
          variant='secondary'
        >
          {app.settings.hotkeyField.recording
            ? 'Press shortcut'
            : app.settings.hotkeyField.displayValue}
        </Button>
      </Field>
      {app.settings.captureSelectFields.map((field) => (
        <Field key={field.key}>
          <Field.Label>{field.label}</Field.Label>
          <Select onValueChange={field.onValueChange} options={field.options} value={field.value} />
        </Field>
      ))}
      {app.settings.numberFields.slice(0, 1).map((field) => (
        <Field key={field.key}>
          <Field.Label>{field.label}</Field.Label>
          <Input min='1' onInput={field.onInput} type='number' value={field.value} />
        </Field>
      ))}
    </div>
    <Switch
      checked={app.settings.toggles.closeToTray.checked}
      onCheckedChange={app.settings.toggles.closeToTray.onCheckedChange}
    >
      Close to tray
    </Switch>
    <Switch
      checked={app.settings.toggles.launchAtLogin.checked}
      onCheckedChange={app.settings.toggles.launchAtLogin.onCheckedChange}
    >
      Launch at login
    </Switch>
  </Panel>
);
