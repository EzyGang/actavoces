export const formatDuration = (seconds: number | null): string => {
  if (seconds === null) {
    return 'In progress';
  }

  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;

  return `${minutes}m ${remainingSeconds.toString().padStart(2, '0')}s`;
};

export const formatTimestamp = (value: string): string => {
  const timestamp = /^\d+$/.test(value) ? Number(value) * 1000 : value;

  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  }).format(new Date(timestamp));
};
