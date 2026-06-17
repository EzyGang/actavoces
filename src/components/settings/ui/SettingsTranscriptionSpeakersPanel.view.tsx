import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { Button } from '../../shared/ui/Button.view';
import { Collapsible } from '../../shared/ui/Collapsible.view';
import { Field } from '../../shared/ui/Field.view';
import { Input } from '../../shared/ui/Input.view';
import { Select } from '../../shared/ui/Select.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';

interface SettingsPanelProps {
  app: ReturnType<typeof useApp>;
}

export const SettingsTranscriptionSpeakersPanel = ({ app }: SettingsPanelProps): JSX.Element => (
  <article class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
    <h2 class='font-semibold text-xl'>Transcription and speakers</h2>
    <div class='grid gap-3 md:grid-cols-2'>
      {app.settings.selectFields.map((field) => (
        <Field key={field.key}>
          <Field.Label>{field.label}</Field.Label>
          <Select onValueChange={field.onValueChange} options={field.options} value={field.value} />
          {field.hint ? (
            <span
              class={
                field.hint.tone === 'warning'
                  ? 'flex flex-1 flex-col gap-1 border border-warning-border bg-warning-bg p-3 text-warning text-xs'
                  : 'flex flex-1 flex-col gap-1 border border-border-base bg-bg-input p-3 text-text-muted text-xs'
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
        </Field>
      ))}
    </div>
    <div class='flex flex-col gap-3 border border-border-base bg-bg-input p-3 text-sm'>
      <Field class='text-sm'>
        <Field.Label>{app.settings.glossaryField.label}</Field.Label>
        <div class='flex gap-2'>
          <Input
            class='min-w-0 flex-1'
            onInput={app.settings.glossaryField.onInput}
            onKeyDown={app.settings.glossaryField.onKeyDown}
            placeholder={app.settings.glossaryField.placeholder}
            type='text'
            value={app.settings.glossaryField.value}
            surface='card'
          />
          <Button class='h-11 px-3' onClick={app.settings.glossaryField.onAdd} variant='secondary'>
            Add
          </Button>
        </div>
        <Field.Description>{app.settings.glossaryField.hint}</Field.Description>
      </Field>
      {app.settings.glossaryField.entries.length > 0 ? (
        <div class='flex flex-wrap gap-2'>
          {app.settings.glossaryField.entries.map((entry) => (
            <span
              class='inline-flex items-center gap-2 border border-border-base bg-bg-card px-2 py-1 font-mono text-xs'
              key={entry.value}
            >
              {entry.value}
              <Button
                aria-label={`Remove ${entry.value}`}
                class='h-auto p-0! text-text-muted hover:text-text-primary'
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
    <Collapsible>
      <Collapsible.Trigger>Advanced speaker options</Collapsible.Trigger>
      <Collapsible.Panel class='grid gap-3 md:grid-cols-3'>
        <Field>
          <Field.Label>{app.settings.speakerCountField.label}</Field.Label>
          <Select
            onValueChange={app.settings.speakerCountField.onValueChange}
            options={app.settings.speakerCountField.options}
            value={app.settings.speakerCountField.value}
          />
        </Field>
        {app.settings.numberFields.slice(1).map((field) => (
          <Field key={field.key}>
            <Field.Label>{field.label}</Field.Label>
            <Input
              min='0'
              onInput={field.onInput}
              type='number'
              value={field.value}
              surface='card'
            />
          </Field>
        ))}
      </Collapsible.Panel>
    </Collapsible>
    {app.settings.draft.value.diarizationBackend === 'pyannote' ? (
      <div class='flex flex-col gap-3 border border-border-base bg-bg-input p-3 text-sm'>
        <Field>
          <Field.Label>{app.settings.huggingFaceTokenField.label}</Field.Label>
          <Input
            onInput={app.settings.huggingFaceTokenField.onInput}
            type='password'
            value={app.settings.huggingFaceTokenField.value}
            surface='card'
          />
        </Field>
        <div class='flex items-center justify-between gap-3'>
          <div class='flex flex-col gap-1'>
            <span class='text-text-muted'>Hugging Face token status</span>
            <span>
              {app.data.snapshot.value.settings.huggingFaceTokenConfigured
                ? 'Saved in local database'
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
    ) : null}
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
      <div class='flex flex-col gap-2 border border-border-base bg-bg-card p-3 text-xs'>
        <div class='flex flex-wrap items-center justify-between gap-2'>
          <span class='text-text-muted'>Recommended model</span>
          <span class='font-mono'>
            {app.data.snapshot.value.settings.modelRecommendation.recommendedModel}
          </span>
        </div>
        <span class='text-text-muted'>
          {app.data.snapshot.value.settings.modelRecommendation.reason}
        </span>
        {app.data.snapshot.value.settings.modelRecommendation.userOverridden ? (
          <span class='border border-warning-border bg-warning-bg p-2 text-warning'>
            Manual override active. The selected model differs from the current recommendation.
          </span>
        ) : null}
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
