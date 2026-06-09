import { describe, expect, it } from 'vitest';
import { formatDuration, formatTimestamp } from '../utils/format';

describe('format helpers', () => {
  it('formats in-progress and completed durations', () => {
    expect(formatDuration(null)).toBe('In progress');
    expect(formatDuration(65)).toBe('1m 05s');
  });

  it('formats unix timestamp strings as local dates', () => {
    const expected = new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    }).format(new Date(1_717_938_012_000));

    expect(formatTimestamp('1717938012')).toBe(expected);
  });
});
