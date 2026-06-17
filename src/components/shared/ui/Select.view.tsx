import { Select as BaseSelect } from '@base-ui/react/select';
import { clsx } from 'clsx';
import type { JSX } from 'preact';

interface SelectOption {
  label: string;
  value: string;
}

interface SelectProps {
  class?: string;
  disabled?: boolean;
  onValueChange: (value: string) => void;
  options: SelectOption[];
  value: string;
}

export const Select = ({
  class: classProp,
  disabled = false,
  onValueChange,
  options,
  value
}: SelectProps): JSX.Element => (
  <BaseSelect.Root<string>
    disabled={disabled}
    items={options}
    onValueChange={(nextValue) => {
      if (nextValue !== null) {
        onValueChange(nextValue);
      }
    }}
    value={value}
  >
    <BaseSelect.Trigger
      className={clsx(
        'flex h-11 w-full items-center justify-between gap-3 border border-border-base bg-bg-input px-3 font-mono text-text-primary text-xs outline-none transition-colors duration-fast hover:border-border-focus focus:border-border-focus focus-visible:ring-2 focus-visible:ring-border-focus disabled:cursor-not-allowed disabled:opacity-50',
        classProp
      )}
    >
      <BaseSelect.Value />
      <BaseSelect.Icon className='text-text-muted'>v</BaseSelect.Icon>
    </BaseSelect.Trigger>
    <BaseSelect.Portal>
      <BaseSelect.Positioner className='z-50 outline-none' sideOffset={4}>
        <BaseSelect.Popup className='max-h-72 min-w-(--anchor-width) overflow-y-auto border border-border-base bg-bg-card p-1 text-text-primary shadow-modal'>
          {options.map((option) => (
            <BaseSelect.Item
              className='flex cursor-pointer items-center justify-between gap-3 px-3 py-2 font-mono text-xs outline-none data-highlighted:bg-bg-hover data-selected:text-text-primary data-disabled:cursor-not-allowed data-disabled:opacity-50'
              key={option.value}
              value={option.value}
            >
              <BaseSelect.ItemText>{option.label}</BaseSelect.ItemText>
              <BaseSelect.ItemIndicator className='text-text-muted'>✓</BaseSelect.ItemIndicator>
            </BaseSelect.Item>
          ))}
        </BaseSelect.Popup>
      </BaseSelect.Positioner>
    </BaseSelect.Portal>
  </BaseSelect.Root>
);
