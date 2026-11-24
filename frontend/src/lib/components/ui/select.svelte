<script lang="ts">
  import { cn } from '$lib/utils';

  interface Option {
    value: string;
    label: string;
  }

  interface Props {
    options: Option[];
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    id?: string;
    /** Accessible name for selects without a visible <label for> association. */
    ariaLabel?: string;
    class?: string;
    onchange?: (value: string) => void;
  }

  let {
    options,
    value = $bindable(''),
    placeholder,
    disabled = false,
    id,
    ariaLabel,
    class: className = '',
    onchange,
  }: Props = $props();

  function handleChange(event: Event): void {
    value = (event.currentTarget as HTMLSelectElement).value;
    onchange?.(value);
  }
</script>

<select
  {id}
  {disabled}
  aria-label={ariaLabel}
  {value}
  onchange={handleChange}
  class={cn(
    'h-9 w-full rounded-md border border-border bg-card px-2 text-sm text-foreground shadow-zf-1',
    'focus-visible:border-primary focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
>
  {#if placeholder}
    <option value="" disabled>{placeholder}</option>
  {/if}
  {#each options as option (option.value)}
    <option value={option.value}>{option.label}</option>
  {/each}
</select>
