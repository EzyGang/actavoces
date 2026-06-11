import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';

interface SettingsPanelProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsGeneralCapturePanel = ({ app }: SettingsPanelProps): JSX.Element => (
  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
    <h2 class='font-semibold text-xl'>General and capture</h2>
    <div class='grid gap-3 md:grid-cols-2'>
      {app.settings.folderFields.map((field) => (
        <label class='flex flex-col gap-2 text-sm' key={field.key}>
          <span class='text-text-muted'>{field.label}</span>
          <div class='flex gap-2'>
            <input
              class='h-11 min-w-0 flex-1 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
              readOnly
              value={field.value}
            />
            <Button class='h-11 px-3' onClick={field.onSelect} type='button' variant='secondary'>
              Choose
            </Button>
          </div>
        </label>
      ))}
      <label class='flex flex-col gap-2 text-sm'>
        <span class='text-text-muted'>{app.settings.hotkeyField.label}</span>
        <button
          class='h-11 border border-border-base bg-bg-input px-3 text-left font-mono text-xs outline-none hover:border-border-focus focus:border-border-focus'
          onClick={app.settings.hotkeyField.onCapture}
          type='button'
        >
          {app.settings.hotkeyField.recording
            ? 'Press shortcut'
            : app.settings.hotkeyField.displayValue}
        </button>
      </label>
      {app.settings.captureSelectFields.map((field) => (
        <label class='flex flex-col gap-2 text-sm' key={field.key}>
          <span class='text-text-muted'>{field.label}</span>
          <select
            class='h-11 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
            onChange={field.onChange}
            value={field.value}
          >
            {field.options.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
      ))}
      {app.settings.numberFields.slice(0, 1).map((field) => (
        <label class='flex flex-col gap-2 text-sm' key={field.key}>
          <span class='text-text-muted'>{field.label}</span>
          <input
            class='h-11 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
            min='1'
            onInput={field.onInput}
            type='number'
            value={field.value}
          />
        </label>
      ))}
    </div>
    <label class='flex items-center gap-3 text-sm'>
      <input
        checked={app.settings.toggles.closeToTray.checked}
        class='h-4 w-4'
        onInput={app.settings.toggles.closeToTray.onInput}
        type='checkbox'
      />
      <span>Close to tray</span>
    </label>
    <label class='flex items-center gap-3 text-sm'>
      <input
        checked={app.settings.toggles.launchAtLogin.checked}
        class='h-4 w-4'
        onInput={app.settings.toggles.launchAtLogin.onInput}
        type='checkbox'
      />
      <span>Launch at login</span>
    </label>
  </article>
);
