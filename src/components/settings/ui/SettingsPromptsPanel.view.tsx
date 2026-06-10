import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';

interface SettingsPanelProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsPromptsPanel = ({ app }: SettingsPanelProps): JSX.Element => (
  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
    <h2 class='font-semibold text-xl'>Prompts</h2>
    {app.settings.textareaFields.map((field) => (
      <label class='flex flex-col gap-2 text-sm' key={field.key}>
        <span class='text-text-muted'>{field.label}</span>
        <textarea
          class='min-h-28 resize-y border border-border-base bg-bg-input p-3 font-mono text-xs outline-none focus:border-border-focus'
          onInput={field.onInput}
          value={field.value}
        />
      </label>
    ))}
  </article>
);
