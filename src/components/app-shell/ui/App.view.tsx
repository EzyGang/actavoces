import type { JSX } from 'preact';
import { DashboardRoute } from '../../dashboard/ui/DashboardRoute.view';
import { JobsRoute } from '../../jobs/ui/JobsRoute.view';
import { RecordingsSection } from '../../recordings/ui/RecordingsSection.view';
import { SettingsRoute } from '../../settings/ui/SettingsRoute.view';
import { SetupRoute } from '../../setup/ui/SetupRoute.view';
import { AppLogo } from '../../shared/ui/AppLogo.view';
import { Button } from '../../shared/ui/Button.view';
import { StatusBadge } from '../../shared/ui/StatusBadge.view';
import type { useApp } from '../hooks/useApp.hook';
import { AppSidebar } from './AppSidebar.view';
import { SortformerSetupToast } from './SortformerSetupToast.view';
import { UnsavedSettingsToast } from './UnsavedSettingsToast.view';

interface AppViewProps {
  app: ReturnType<typeof useApp>;
}

export const AppView = ({ app }: AppViewProps): JSX.Element =>
  app.status.setupReady.value ? (
    <main class='min-h-screen overflow-hidden bg-bg-page text-text-primary'>
      <section class='grid min-h-screen grid-rows-[64px_minmax(0,1fr)]'>
        <header class='flex items-center justify-between border-border-base border-b bg-bg-page px-5'>
          <div class='flex items-center gap-4'>
            <AppLogo class='h-10 w-10' />
            <div class='flex flex-col gap-0.5'>
              <span class='font-semibold text-sm uppercase tracking-wider'>ActaVoces</span>
              <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
                {app.data.routeLabel[app.status.activeRoute.value]}
              </span>
            </div>
          </div>

          <div class='flex min-w-0 items-center gap-3'>
            <div class='hidden shrink-0 items-center gap-2 border border-border-base bg-bg-card px-3 py-2 sm:flex'>
              <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
                Hotkey
              </span>
              <span class='whitespace-nowrap font-mono text-xs'>
                {app.data.displayHotkey(app.data.snapshot.value.settings.hotkey)}
              </span>
            </div>
            <StatusBadge
              label={app.status.isRecording.value ? 'Recording' : 'Ready'}
              status={app.status.isRecording.value ? 'recording' : 'idle'}
            />
            <Button
              disabled={app.status.loading.value}
              onClick={
                app.status.isRecording.value
                  ? app.actions.stopRecording
                  : app.actions.startRecording
              }
              variant={app.status.isRecording.value ? 'secondary' : 'primary'}
            >
              {app.status.isRecording.value ? 'Stop capture' : 'Start capture'}
            </Button>
          </div>
        </header>

        <div class='grid min-h-0 grid-cols-[176px_minmax(0,1fr)] xl:grid-cols-[176px_minmax(0,1fr)_360px]'>
          <nav class='flex min-h-0 flex-col gap-2 border-border-base border-r bg-bg-page p-3'>
            {app.navigation.map((item) => (
              <button
                aria-current={item.isActive ? 'page' : undefined}
                class={
                  item.isActive
                    ? 'border border-text-primary bg-text-primary px-3 py-3 text-left font-semibold text-bg-page text-xs uppercase tracking-wider'
                    : 'border border-border-base bg-bg-card px-3 py-3 text-left font-semibold text-text-secondary text-xs uppercase tracking-wider hover:border-text-muted hover:bg-bg-hover hover:text-text-primary'
                }
                key={item.route}
                onClick={item.onSelect}
                type='button'
              >
                {item.label}
              </button>
            ))}
          </nav>

          <section class='min-h-0 overflow-y-auto bg-bg-page'>
            <div class='flex flex-col gap-5 p-5 lg:p-7'>
              {app.status.error.value ? (
                <section class='border border-error-border bg-error-bg p-4 text-error text-sm'>
                  {app.status.error.value}
                </section>
              ) : null}

              {app.status.activeRoute.value === 'dashboard' ? <DashboardRoute app={app} /> : null}
              {app.status.activeRoute.value === 'recordings' ? (
                <RecordingsSection app={app} />
              ) : null}
              {app.status.activeRoute.value === 'jobs' ? <JobsRoute app={app} /> : null}
              {app.status.activeRoute.value === 'settings' ? <SettingsRoute app={app} /> : null}
            </div>
          </section>

          <AppSidebar app={app} />
        </div>

        {app.status.isRecording.value ? (
          <div class='fixed right-4 bottom-4 flex items-center gap-3 border border-error-border bg-bg-page px-4 py-3 text-error'>
            <span class='h-2.5 w-2.5 bg-error' />
            <span class='font-mono text-xs uppercase tracking-wider'>ActaVoces is recording</span>
          </div>
        ) : null}

        {app.data.sortformerProgress.value ? (
          <SortformerSetupToast progress={app.data.sortformerProgress.value} />
        ) : null}

        {app.status.hasUnsavedSettings.value ? (
          <UnsavedSettingsToast
            offset={app.data.sortformerProgress.value !== null}
            onSave={app.actions.saveSettings}
            saving={app.status.savingSettings.value}
          />
        ) : null}
      </section>
    </main>
  ) : (
    <SetupRoute app={app} />
  );
