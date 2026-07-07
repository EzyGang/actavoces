import { describe, expect, it } from 'vitest';
import type { Artifact, ArtifactKind } from '../types/desktop';

const artifact = (kind: ArtifactKind, label: string, path: string): Artifact => ({
  kind,
  label,
  path,
  ready: true
});

describe('artifact contracts', () => {
  it('accepts clean transcripts and orders readable transcript before raw transcript', () => {
    const primaryReadableArtifacts = [
      artifact('cleanTranscript', 'Clean transcript', '/records/meeting/clean-transcript.md'),
      artifact('rawTranscript', 'Raw ASR transcript', '/records/meeting/meta/raw-transcript.md')
    ] satisfies Artifact[];

    const cleanTranscriptKind: ArtifactKind = primaryReadableArtifacts[0].kind;

    expect(cleanTranscriptKind).toBe('cleanTranscript');
    expect(primaryReadableArtifacts.map((item) => item.kind)).toEqual([
      'cleanTranscript',
      'rawTranscript'
    ]);
  });
});
