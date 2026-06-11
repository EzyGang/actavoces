const modifierLabels: Record<string, string> = {
  Alt: 'Alt',
  Command: 'Cmd',
  Control: 'Ctrl',
  Meta: 'Cmd',
  Shift: 'Shift'
};

export const displayHotkey = (hotkey: string, platform = navigator.platform): string => {
  const isApplePlatform = /Mac|iPhone|iPad|iPod/i.test(platform);

  return hotkey
    .split('+')
    .map((part) => {
      if (part === 'CommandOrControl') {
        return isApplePlatform ? 'Cmd' : 'Ctrl';
      }

      return modifierLabels[part] ?? part;
    })
    .join('+');
};
