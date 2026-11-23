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
    outline: 'border border-input text-muted-foreground',
    success: 'bg-success/15 text-success ring-1 ring-success/30',
    warning: 'bg-warning/15 text-warning ring-1 ring-warning/30',
    error: 'bg-destructive/15 text-destructive ring-1 ring-destructive/30',
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
