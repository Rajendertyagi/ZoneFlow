<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    label?: string;
    /** Id of a visible label element, when associated instead of `label`. */
    labelledby?: string;
    id?: string;
    class?: string;
    onchange?: (checked: boolean) => void;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    label,
    labelledby,
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

<!-- A native <button type="button"> already fires `click` for both Space and
     Enter, so keyboard toggling needs no custom keydown handler (which would
     double-toggle). -->
<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={labelledby ? undefined : label}
  aria-labelledby={labelledby}
  {id}
  {disabled}
  onclick={toggle}
  class={cn(
    'relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors motion-reduce:transition-none',
    'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary',
    checked ? 'bg-primary' : 'bg-input',
    disabled && 'cursor-not-allowed opacity-50',
    className,
  )}
>
  <!-- Contrast-safe thumb: light-on-dark when off, primary-foreground on the
       primary track when on. Both pairs clear AA in every preset/mode. -->
  <span
    class={cn(
      'inline-block h-4 w-4 rounded-full shadow transition-transform motion-reduce:transition-none',
      checked ? 'translate-x-4 bg-primary-foreground' : 'translate-x-0.5 bg-foreground',
    )}
  ></span>
</button>
