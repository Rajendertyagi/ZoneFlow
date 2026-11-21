<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    value?: string;
    placeholder?: string;
    type?: string;
    disabled?: boolean;
    id?: string;
    class?: string;
    oninput?: (event: Event) => void;
    onchange?: (event: Event) => void;
  }

  let {
    value = $bindable(''),
    placeholder,
    type = 'text',
    disabled = false,
    id,
    class: className = '',
    oninput,
    onchange,
  }: Props = $props();
</script>

<input
  {id}
  {type}
  {placeholder}
  {disabled}
  {oninput}
  {onchange}
  bind:value
  class={cn(
    'h-9 w-full rounded-md border border-zf-border bg-zf-surface px-3 text-sm text-zf-text',
    'placeholder:text-zf-faint focus-visible:border-zf-accent focus-visible:outline-none',
    'disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
/>
