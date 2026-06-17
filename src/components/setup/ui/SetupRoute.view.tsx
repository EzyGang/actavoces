import type { JSX } from 'preact';
import type { useApp } from '../../app-shell/hooks/useApp.hook';
import { AppLogo } from '../../shared/ui/AppLogo.view';
import { Button } from '../../shared/ui/Button.view';
import { Field } from '../../shared/ui/Field.view';
import { Input } from '../../shared/ui/Input.view';

interface SetupRouteProps {
  app: ReturnType<typeof useApp>;
}

export const SetupRoute = ({ app }: SetupRouteProps): JSX.Element => (
  <main class='flex min-h-screen items-center justify-center bg-bg-page p-6 text-text-primary'>
    <section class='flex w-full max-w-xl flex-col gap-6 border border-border-base bg-bg-card p-6'>
      <div class='flex items-center gap-4'>
        <AppLogo class='h-11 w-11' />
        <div class='flex flex-col gap-1'>
          <span class='font-semibold text-sm uppercase tracking-wider'>ActaVoces</span>
          <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
            Local worker setup
          </span>
        </div>
      </div>

      <div class='flex flex-col gap-3'>
        <h1 class='font-semibold text-2xl'>
          {app.status.needsDiarizationSetup.value
            ? 'Set up speaker diarization'
            : 'Preparing transcription runtime'}
        </h1>
        <p class='text-sm text-text-secondary'>
          {app.status.needsDiarizationSetup.value
            ? 'Speaker labels require local pyannote.audio, ffmpeg, accepted Hugging Face model terms, and a Hugging Face token.'
            : app.data.setupProgress.value.step}
        </p>
      </div>

      {app.status.needsDiarizationSetup.value ? (
        <section class='flex flex-col gap-4 border border-warning-border bg-warning-bg p-4 text-warning text-sm'>
          <div class='flex flex-col gap-2'>
            <span class='font-semibold'>Full functionality needs speaker diarization setup.</span>
            <span>
              Accept the pyannote model terms on Hugging Face, create an access token, then install
              the local diarization runtime.
            </span>
            <span class='flex flex-wrap gap-3 text-xs'>
              <a
                class='text-text-primary underline'
                href='https://huggingface.co/pyannote/speaker-diarization-community-1'
                rel='noreferrer'
                target='_blank'
              >
                Model terms
              </a>
              <a
                class='text-text-primary underline'
                href='https://huggingface.co/settings/tokens'
                rel='noreferrer'
                target='_blank'
              >
                Token settings
              </a>
            </span>
          </div>
          <Field>
            <Field.Label class='text-warning'>Hugging Face token</Field.Label>
            <Input
              onInput={app.settings.huggingFaceTokenField.onInput}
              type='password'
              value={app.settings.huggingFaceTokenField.value}
            />
          </Field>
        </section>
      ) : null}

      <div class='h-2 border border-border-base bg-bg-input'>
        <div
          class={
            app.data.setupProgress.value.status === 'failed'
              ? 'h-full w-full bg-error'
              : app.data.setupProgress.value.status === 'ready'
                ? 'h-full w-full bg-success'
                : 'h-full w-2/3 bg-text-primary'
          }
        />
      </div>

      {app.data.setupProgress.value.error ? (
        <section class='max-w-full overflow-x-auto break-all whitespace-pre-wrap border border-error-border bg-error-bg p-4 text-error text-sm'>
          {app.data.setupProgress.value.error}
        </section>
      ) : null}

      <div class='flex items-center justify-between gap-4 text-text-muted text-xs uppercase tracking-wider'>
        <span>{app.data.setupProgress.value.status}</span>
        {app.status.needsDiarizationSetup.value ? (
          <div class='flex gap-3'>
            <Button
              disabled={app.status.setupRunning.value}
              onClick={app.actions.skipDiarizationSetup}
              variant='ghost'
            >
              Skip for now
            </Button>
            <Button
              disabled={app.status.setupRunning.value}
              onClick={app.actions.setupDiarization}
              variant='secondary'
            >
              Set up diarization
            </Button>
          </div>
        ) : app.data.setupProgress.value.status === 'failed' ? (
          <Button
            disabled={app.status.setupRunning.value}
            onClick={app.actions.retrySetup}
            variant='secondary'
          >
            Retry
          </Button>
        ) : null}
      </div>
    </section>
  </main>
);
