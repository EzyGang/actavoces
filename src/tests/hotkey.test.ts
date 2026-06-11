import { describe, expect, it } from 'vitest';
import { displayHotkey } from '../utils/hotkey';

describe('displayHotkey', () => {
  it('uses Ctrl for CommandOrControl on Windows', () => {
    expect(displayHotkey('CommandOrControl+Shift+Space', 'Win32')).toBe('Ctrl+Shift+Space');
  });

  it('uses Cmd for CommandOrControl on macOS', () => {
    expect(displayHotkey('CommandOrControl+Shift+Space', 'MacIntel')).toBe('Cmd+Shift+Space');
  });

  it('shortens explicit modifier names', () => {
    expect(displayHotkey('Control+Alt+Meta+K', 'Win32')).toBe('Ctrl+Alt+Cmd+K');
  });
});
