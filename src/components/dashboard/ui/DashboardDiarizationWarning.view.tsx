import type { JSX } from 'preact';

export const DashboardDiarizationWarning = (): JSX.Element => (
  <section class='border border-warning-border bg-warning-bg p-4 text-warning text-sm'>
    Speaker diarization is not fully set up. Recordings will still transcribe, but speaker labels
    need pyannote setup from Settings.
  </section>
);
