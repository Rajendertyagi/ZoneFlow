<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    label?: string;
    id?: string;
    class?: string;
    onchange?: (checked: boolean) => void;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    label,
    id,
    class: className = '',
    onchange,
  }: Props = $props();

  function toggle(): void {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {id}
  {disabled}
  onclick={toggle}
  class={cn(
    'relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors',
    'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-zf-accent',
    checked ? 'bg-zf-accent' : 'bg-zf-border-strong',
    disabled && 'cursor-not-allowed opacity-50',
    className,
  )}
>
  <span
    class={cn(
      'inline-block h-4 w-4 rounded-full bg-white shadow transition-transform',
      checked ? 'translate-x-4' : 'translate-x-0.5',
    )}
  ></span>
</button>
