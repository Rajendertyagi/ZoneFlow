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
    default: 'bg-zf-raised text-zf-text',
    outline: 'border border-zf-border-strong text-zf-muted',
    success: 'bg-zf-ok/15 text-zf-ok ring-1 ring-zf-ok/30',
    warning: 'bg-zf-warn/15 text-zf-warn ring-1 ring-zf-warn/30',
    error: 'bg-zf-error/15 text-zf-error ring-1 ring-zf-error/30',
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
