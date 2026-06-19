import {
  IconBrandGithub,
  IconBrandX,
  IconBug,
  IconDownload,
  IconExternalLink,
  IconMessage
} from '@tabler/icons-react';
import type { JSX } from 'preact';
import { AppLogo } from '../components/shared/ui/AppLogo.view';
import '../App.css';
import dashboardScreenshot from '../../screenshots/1.png';
import {
  appVersion,
  artifactItems,
  creatorUrl,
  downloadRows,
  faqItems,
  featureCards,
  feedbackUrl,
  issuesUrl,
  releasesUrl,
  repositoryUrl,
  workflowSteps
} from './landing.content';

export const LandingPage = (): JSX.Element => (
  <main class='min-h-screen bg-bg-page text-text-primary'>
    <header class='sticky top-0 z-20 border-border-base border-b bg-bg-page/95 backdrop-blur'>
      <nav class='mx-auto flex max-w-7xl items-center justify-between gap-4 px-5 py-4 lg:px-8'>
        <a class='flex items-center gap-3' href='#top'>
          <AppLogo class='h-9 w-9' />
          <span class='flex flex-col gap-0.5'>
            <span class='font-semibold text-sm uppercase tracking-wider'>ActaVoces</span>
            <span class='font-mono text-text-muted text-[10px] uppercase tracking-wider'>
              Local-first recorder
            </span>
          </span>
        </a>
        <div class='hidden items-center gap-3 md:flex'>
          <a
            class='text-text-secondary text-xs uppercase tracking-wider hover:text-text-primary'
            href='#features'
          >
            Features
          </a>
          <a
            class='text-text-secondary text-xs uppercase tracking-wider hover:text-text-primary'
            href='#privacy'
          >
            Privacy
          </a>
          <a
            class='text-text-secondary text-xs uppercase tracking-wider hover:text-text-primary'
            href='#faq'
          >
            FAQ
          </a>
          <a
            class='inline-flex items-center gap-2 border border-border-base px-4 py-2 font-semibold text-text-primary text-xs uppercase tracking-wider hover:border-text-muted hover:bg-bg-hover'
            href={repositoryUrl}
            rel='noreferrer'
            target='_blank'
          >
            <IconBrandGithub aria-hidden='true' className='h-4 w-4' />
            GitHub
          </a>
        </div>
      </nav>
    </header>

    <section class='mx-auto flex max-w-7xl flex-col gap-10 px-5 py-12 lg:px-8 lg:py-20' id='top'>
      <div class='grid items-center gap-8 lg:grid-cols-[minmax(0,1fr)_420px]'>
        <div class='flex flex-col gap-7'>
          <div class='flex flex-wrap gap-2'>
            <span class='border border-success-border bg-success-bg px-3 py-1 font-mono text-[11px] text-success uppercase tracking-wider'>
              Local-first
            </span>
            <span class='border border-border-base bg-bg-card px-3 py-1 font-mono text-[11px] text-text-muted uppercase tracking-wider'>
              Tauri desktop
            </span>
            <span class='border border-border-base bg-bg-card px-3 py-1 font-mono text-[11px] text-text-muted uppercase tracking-wider'>
              AGPL-3.0
            </span>
          </div>
          <div class='flex flex-col gap-5'>
            <h1 class='max-w-5xl font-semibold text-5xl leading-[0.95] tracking-[-0.045em] md:text-7xl lg:text-8xl'>
              Meeting recordings that stay inspectable.
            </h1>
            <p class='max-w-3xl text-lg text-text-secondary leading-8 md:text-xl'>
              ActaVoces records meetings, transcribes locally by default, adds speaker labels, and
              writes Markdown plus JSON artifacts beside each recording.
            </p>
          </div>
          <div class='flex flex-col gap-3 sm:flex-row'>
            <a
              class='inline-flex h-12 items-center justify-center gap-2 bg-text-primary px-5 font-semibold text-bg-page text-xs uppercase tracking-wider hover:opacity-90'
              href={releasesUrl}
              rel='noreferrer'
              target='_blank'
            >
              <IconDownload aria-hidden='true' className='h-4 w-4' />
              Download releases
            </a>
            <a
              class='inline-flex h-12 items-center justify-center gap-2 border border-border-base px-5 font-semibold text-text-primary text-xs uppercase tracking-wider hover:border-text-muted hover:bg-bg-hover'
              href={repositoryUrl}
              rel='noreferrer'
              target='_blank'
            >
              <IconBrandGithub aria-hidden='true' className='h-4 w-4' />
              View source
            </a>
          </div>
        </div>
        <aside class='flex flex-col gap-5 border border-border-base bg-bg-card p-5'>
          <div class='flex flex-col gap-1'>
            <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
              Current status
            </span>
            <span class='font-mono text-xs text-success'>{appVersion}</span>
          </div>
          <p class='text-text-secondary text-sm leading-6'>
            ActaVoces is used today. Windows has the most runtime, macOS has lighter runtime, and
            Linux lacks active QA.
          </p>
          <div class='grid gap-2 text-sm'>
            <a
              class='inline-flex items-center gap-2 text-text-primary underline-offset-4 hover:underline'
              href={feedbackUrl}
              rel='noreferrer'
              target='_blank'
            >
              <IconMessage aria-hidden='true' className='h-4 w-4 text-text-muted' />
              Suggest feedback
            </a>
            <a
              class='inline-flex items-center gap-2 text-text-primary underline-offset-4 hover:underline'
              href={issuesUrl}
              rel='noreferrer'
              target='_blank'
            >
              <IconBug aria-hidden='true' className='h-4 w-4 text-text-muted' />
              Report an issue
            </a>
          </div>
        </aside>
      </div>

      <section class='grid items-center gap-10 border border-border-base bg-bg-page p-6 lg:grid-cols-[0.48fr_1fr] lg:gap-12 lg:p-10'>
        <div class='flex max-w-md flex-col gap-8 justify-self-center lg:justify-self-start'>
          <div class='flex items-center gap-3'>
            <AppLogo class='h-10 w-10' />
            <div class='flex flex-col gap-0.5'>
              <span class='font-semibold text-sm uppercase tracking-wider'>ActaVoces</span>
              <span class='font-mono text-text-muted text-[10px] uppercase tracking-wider'>
                Records. Voices. Files.
              </span>
            </div>
          </div>
          <div class='flex flex-col gap-5'>
            <div class='flex h-12 items-end gap-1 border-border-base border-b pb-3'>
              <span class='h-4 w-2 bg-text-primary' />
              <span class='h-7 w-2 bg-text-secondary' />
              <span class='h-5 w-2 bg-text-muted' />
              <span class='h-9 w-2 bg-text-primary' />
              <span class='h-5 w-2 bg-text-muted' />
              <span class='h-8 w-2 bg-text-secondary' />
              <span class='h-3 w-2 bg-text-muted' />
              <span class='h-6 w-2 bg-text-primary' />
            </div>
            <p class='max-w-md text-base text-text-secondary leading-7'>
              Local-first desktop app for recording, transcribing, diarizing, and summarizing
              meetings and conversations.
            </p>
          </div>
        </div>
        <img
          alt='ActaVoces dashboard showing capture and processing pipeline'
          class='w-full bg-bg-page lg:justify-self-end'
          src={dashboardScreenshot}
        />
      </section>
    </section>

    <section class='border-border-base border-y bg-bg-card' id='features'>
      <div class='mx-auto flex max-w-7xl flex-col gap-8 px-5 py-14 lg:px-8'>
        <div class='flex max-w-3xl flex-col gap-3'>
          <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
            Features
          </span>
          <h2 class='font-semibold text-3xl tracking-[-0.03em] md:text-5xl'>
            Files you can inspect.
          </h2>
        </div>
        <div class='grid gap-4 md:grid-cols-2 xl:grid-cols-3'>
          {featureCards.map((feature) => (
            <article
              class='flex min-h-48 flex-col gap-8 border border-border-base bg-bg-page p-5'
              key={feature.title}
            >
              <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
                {feature.label}
              </span>
              <div class='flex flex-col gap-2'>
                <h3 class='font-semibold text-xl'>{feature.title}</h3>
                <p class='text-sm text-text-secondary leading-6'>{feature.text}</p>
              </div>
            </article>
          ))}
        </div>
      </div>
    </section>

    <section
      class='mx-auto grid max-w-7xl gap-4 px-5 py-14 lg:grid-cols-[0.9fr_1.1fr] lg:px-8'
      id='privacy'
    >
      <article class='flex flex-col gap-6 border border-border-base bg-bg-card p-5'>
        <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>Privacy</span>
        <h2 class='font-semibold text-3xl tracking-[-0.03em]'>
          Local recording and processing defaults.
        </h2>
        <p class='text-text-secondary leading-7'>
          Recording, storage, transcription, and Sortformer speaker labels run locally by default.
          Network access covers model downloads and networked summary providers.
        </p>
        <p class='text-text-secondary leading-7'>
          Summaries can point at local Ollama endpoints or any OpenAI-compatible API. They stay off
          until enabled and configured.
        </p>
      </article>
      <article class='grid gap-4 md:grid-cols-2'>
        <div class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
          <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
            Workflow
          </span>
          <div class='flex flex-col gap-3'>
            {workflowSteps.map((step, index) => (
              <div class='flex gap-3 border border-border-base bg-bg-input p-3' key={step}>
                <span class='font-mono text-text-muted text-xs'>{index + 1}</span>
                <span class='text-sm text-text-secondary'>{step}</span>
              </div>
            ))}
          </div>
        </div>
        <div class='flex flex-col gap-4 border border-border-base bg-bg-card p-5'>
          <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
            Artifacts
          </span>
          <div class='flex flex-col gap-2'>
            {artifactItems.map((item) => (
              <span
                class='border border-border-base bg-bg-input px-3 py-2 font-mono text-text-secondary text-xs'
                key={item}
              >
                {item}
              </span>
            ))}
          </div>
        </div>
      </article>
    </section>

    <section class='border-border-base border-y bg-bg-card' id='download'>
      <div class='mx-auto grid max-w-7xl gap-6 px-5 py-14 lg:grid-cols-[0.8fr_1.2fr] lg:px-8'>
        <div class='flex flex-col gap-4'>
          <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
            Download
          </span>
          <h2 class='font-semibold text-3xl tracking-[-0.03em]'>Builds live on GitHub Releases.</h2>
          <a
            class='inline-flex h-12 w-fit items-center justify-center gap-2 bg-text-primary px-5 font-semibold text-bg-page text-xs uppercase tracking-wider hover:opacity-90'
            href={releasesUrl}
            rel='noreferrer'
            target='_blank'
          >
            <IconExternalLink aria-hidden='true' className='h-4 w-4' />
            Open releases
          </a>
        </div>
        <div class='flex flex-col gap-3'>
          {downloadRows.map((download) => (
            <div
              class='grid gap-2 border border-border-base bg-bg-page p-4 sm:grid-cols-[220px_minmax(0,1fr)]'
              key={download.platform}
            >
              <span class='font-semibold'>{download.platform}</span>
              <span class='flex flex-wrap gap-3 font-mono text-xs'>
                {download.links.map((link) => (
                  <a
                    class='inline-flex items-center gap-1.5 text-text-secondary hover:text-text-primary'
                    href={link.href}
                    key={link.href}
                    rel='noreferrer'
                    target='_blank'
                  >
                    <IconDownload aria-hidden='true' className='h-3.5 w-3.5' />
                    {link.label}
                  </a>
                ))}
              </span>
            </div>
          ))}
          <p class='border border-warning-border bg-warning-bg p-4 text-sm text-warning leading-6'>
            Builds are unsigned today. Code signing is expensive for an independent open-source app.
            macOS signing may come later. Windows will likely stay unsigned for a while.
          </p>
        </div>
      </div>
    </section>

    <section class='mx-auto flex max-w-7xl flex-col gap-8 px-5 py-14 lg:px-8' id='faq'>
      <div class='flex max-w-3xl flex-col gap-3'>
        <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>FAQ</span>
        <h2 class='font-semibold text-3xl tracking-[-0.03em] md:text-5xl'>Common questions.</h2>
      </div>
      <div class='grid gap-4 md:grid-cols-2'>
        {faqItems.map((item) => (
          <article
            class='flex flex-col gap-3 border border-border-base bg-bg-card p-5'
            key={item.question}
          >
            <h3 class='font-semibold text-lg'>{item.question}</h3>
            <p class='text-sm text-text-secondary leading-6'>{item.answer}</p>
          </article>
        ))}
      </div>
    </section>

    <footer class='border-border-base border-t bg-bg-card'>
      <div class='mx-auto flex max-w-7xl flex-col justify-between gap-5 px-5 py-8 md:flex-row md:items-center lg:px-8'>
        <div class='flex items-center gap-3'>
          <AppLogo class='h-8 w-8' />
          <span class='font-mono text-text-muted text-[11px] uppercase tracking-wider'>
            ActaVoces is AGPL-3.0-or-later.
          </span>
        </div>
        <div class='flex flex-wrap gap-4 text-xs uppercase tracking-wider'>
          <a
            class='inline-flex items-center gap-1.5 text-text-secondary hover:text-text-primary'
            href={repositoryUrl}
            rel='noreferrer'
            target='_blank'
          >
            <IconBrandGithub aria-hidden='true' className='h-4 w-4' />
            GitHub
          </a>
          <a
            class='inline-flex items-center gap-1.5 text-text-secondary hover:text-text-primary'
            href={issuesUrl}
            rel='noreferrer'
            target='_blank'
          >
            <IconBug aria-hidden='true' className='h-4 w-4' />
            Issues
          </a>
          <a
            class='inline-flex items-center gap-1.5 text-text-secondary hover:text-text-primary'
            href={feedbackUrl}
            rel='noreferrer'
            target='_blank'
          >
            <IconMessage aria-hidden='true' className='h-4 w-4' />
            Feedback
          </a>
          <a
            class='inline-flex items-center gap-1.5 text-text-secondary hover:text-text-primary'
            href={creatorUrl}
            rel='noreferrer'
            target='_blank'
          >
            <IconBrandX aria-hidden='true' className='h-4 w-4' />
            Built by galtozzy
          </a>
        </div>
      </div>
    </footer>
  </main>
);
