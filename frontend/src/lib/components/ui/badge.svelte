<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  type Variant = 'default' | 'outline' | 'success' | 'warning' | 'error';

  interface Props {
    variant?: Variant;
    class?: string;
    children?: Snippet;
  }

  let { variant = 'default', class: className = '', children }: Props = $props();

  const variants: Record<Variant, string> = {
    default: 'bg-secondary text-foreground',
    // `outline` keeps its border: it marks the selected profile/zone and is a
    // meaningful state boundary, not decorative chrome.
    outline: 'border border-input text-muted-foreground',
    // Status badges are tinted surfaces; the tint + semantic text colour keep
    // the state legible without a boxed ring.
    success: 'bg-success/15 text-success',
    warning: 'bg-warning/15 text-warning',
    error: 'bg-destructive/15 text-destructive',
  };
</script>

<span
  class={cn(
    'inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[0.7rem] font-medium leading-none',
    variants[variant],
    className,
  )}
>
  {@render children?.()}
</span>
