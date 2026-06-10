import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';

interface SettingsPanelProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsTranscriptionSpeakersPanel = ({ app }: SettingsPanelProps): JSX.Element => (
  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
    <h2 class='font-semibold text-xl'>Transcription and speakers</h2>
    <div class='grid gap-3 md:grid-cols-2'>
      {app.settings.selectFields.map((field) => (
        <label class='flex flex-col gap-2 text-sm' key={field.key}>
          <span class='text-text-muted'>{field.label}</span>
          <select
            class='h-11 border border-border-base bg-bg-input px-3 font-mono text-xs outline-none focus:border-border-focus'
            onChange={field.onChange}
            value={field.value}
          >
            {field.options.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
          {field.hint ? (
            <span
              class={
                field.hint.tone === 'warning'
                  ? 'flex flex-col gap-1 border border-warning-border bg-warning-bg p-3 text-warning text-xs'
                  : 'flex flex-col gap-1 border border-border-base bg-bg-input p-3 text-text-muted text-xs'
              }
            >
              {field.hint.title ? <span class='font-semibold'>{field.hint.title}</span> : null}
              <span>{field.hint.text}</span>
              {field.hint.links ? (
                <span class='flex flex-wrap gap-2'>
                  {field.hint.links.map((link) => (
                    <a
                      class='text-text-primary underline'
                      href={link.href}
                      key={link.href}
                      rel='noreferrer'
                      target='_blank'
                    >
                      {link.label}
                    </a>
                  ))}
                </span>
              ) : null}
            </span>
          ) : null}
        </label>
      ))}
    </div>
    <details class='border border-border-base bg-bg-input p-3'>
      <summary class='cursor-pointer text-text-secondary text-sm'>Advanced speaker options</summary>
      <div class='grid gap-3 pt-3 md:grid-cols-3'>
        {app.settings.numberFields.slice(1).map((field) => (
          <label class='flex flex-col gap-2 text-sm' key={field.key}>
            <span class='text-text-muted'>{field.label}</span>
            <input
              class='h-11 border border-border-base bg-bg-card px-3 font-mono text-xs outline-none focus:border-border-focus'
              min='0'
              onInput={field.onInput}
              type='number'
              value={field.value}
            />
          </label>
        ))}
      </div>
    </details>
    <div class='flex flex-col gap-3 border border-border-base bg-bg-input p-3 text-sm'>
      <label class='flex flex-col gap-2'>
        <span class='text-text-muted'>{app.settings.huggingFaceTokenField.label}</span>
        <input
          class='h-11 border border-border-base bg-bg-card px-3 font-mono text-xs outline-none focus:border-border-focus'
          onInput={app.settings.huggingFaceTokenField.onInput}
          type='password'
          value={app.settings.huggingFaceTokenField.value}
        />
      </label>
      <div class='flex items-center justify-between gap-3'>
        <div class='flex flex-col gap-1'>
          <span class='text-text-muted'>Hugging Face token status</span>
          <span>
            {app.data.snapshot.value.settings.huggingFaceTokenConfigured
              ? 'Saved in keychain'
              : 'Missing'}
          </span>
        </div>
        <Button
          class='h-9 px-3'
          disabled={
            app.status.savingSettings.value ||
            !app.data.snapshot.value.settings.huggingFaceTokenConfigured
          }
          onClick={app.actions.clearHuggingFaceToken}
          variant='ghost'
        >
          Clear token
        </Button>
      </div>
    </div>
    <div class='flex flex-col gap-3 border border-border-base bg-bg-input p-3 text-sm'>
      <div class='flex items-center justify-between gap-3'>
        <div class='flex flex-col gap-1'>
          <span class='text-text-muted'>Selected model</span>
          <span class='font-mono text-xs'>{app.settings.draft.value.whisperModel}</span>
        </div>
        <StatusBadge
          label={
            app.data.selectedModel.value?.installed
              ? 'Installed'
              : app.data.selectedModel.value?.setupRequired
                ? 'Setup required'
                : 'Missing'
          }
          status={
            app.data.selectedModel.value?.installed
              ? 'complete'
              : app.data.selectedModel.value?.setupRequired
                ? 'needsSetup'
                : 'pending'
          }
        />
      </div>
      <div class='grid gap-2 sm:grid-cols-2'>
        <Button
          class='h-9 px-3'
          disabled={app.status.loading.value}
          onClick={app.actions.refreshModels}
          variant='ghost'
        >
          Refresh models
        </Button>
        <Button
          class='h-9 px-3'
          disabled={app.status.installingModel.value || app.data.selectedModel.value?.installed}
          onClick={app.actions.installSelectedModel}
          variant='secondary'
        >
          Install selected
        </Button>
      </div>
      {app.data.snapshot.value.models.length > 0 ? (
        <div class='grid gap-2'>
          {app.data.snapshot.value.models.map((model) => (
            <div
              class='flex items-center justify-between gap-3 border border-border-base bg-bg-card px-3 py-2'
              key={model.name}
            >
              <span class='font-mono text-xs'>{model.name}</span>
              <span class='text-text-muted text-xs'>
                {model.installed
                  ? 'installed'
                  : model.setupRequired
                    ? `${model.dependency} required`
                    : 'not installed'}
              </span>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  </article>
);
