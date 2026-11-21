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
    class?: string;
    onchange?: (value: string) => void;
  }

  let {
    options,
    value = $bindable(''),
    placeholder,
    disabled = false,
    id,
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
  {value}
  onchange={handleChange}
  class={cn(
    'h-9 w-full rounded-md border border-zf-border bg-zf-surface px-2 text-sm text-zf-text',
    'focus-visible:border-zf-accent focus-visible:outline-none disabled:opacity-50',
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
