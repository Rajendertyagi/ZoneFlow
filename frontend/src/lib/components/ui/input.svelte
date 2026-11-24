<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    value?: string;
    placeholder?: string;
    type?: string;
    disabled?: boolean;
    id?: string;
    /** Accessible name for inputs without a visible <label for> association. */
    ariaLabel?: string;
    /** Optional invalid state for form validation messaging. */
    ariaInvalid?: boolean;
    /** Id of a description/error element for screen readers. */
    ariaDescribedby?: string;
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
    ariaLabel,
    ariaInvalid,
    ariaDescribedby,
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
  aria-label={ariaLabel}
  aria-invalid={ariaInvalid}
  aria-describedby={ariaDescribedby}
  {oninput}
  {onchange}
  bind:value
  class={cn(
    'h-9 w-full rounded-md border border-border bg-card px-3 text-sm text-foreground shadow-zf-1',
    'placeholder:text-muted-foreground-faint focus-visible:border-primary focus-visible:outline-none',
    'disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
/>
