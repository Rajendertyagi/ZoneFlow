<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  type Variant = 'default' | 'outline' | 'ghost' | 'destructive';
  type Size = 'sm' | 'default' | 'icon';

  interface Props {
    variant?: Variant;
    size?: Size;
    disabled?: boolean;
    type?: 'button' | 'submit' | 'reset';
    class?: string;
    title?: string;
    onclick?: (event: MouseEvent) => void;
    children?: Snippet;
  }

  let {
    variant = 'default',
    size = 'default',
    disabled = false,
    type = 'button',
    class: className = '',
    title,
    onclick,
    children,
  }: Props = $props();

  const variants: Record<Variant, string> = {
    default: 'bg-zf-accent text-zf-accent-ink hover:bg-zf-accent-hover',
    outline: 'border border-zf-border-strong bg-transparent text-zf-text hover:bg-zf-raised',
    ghost: 'bg-transparent text-zf-muted hover:bg-zf-raised hover:text-zf-text',
    destructive: 'bg-zf-error text-white hover:brightness-110',
  };
  const sizes: Record<Size, string> = {
    sm: 'h-7 px-2.5 text-xs',
    default: 'h-9 px-4',
    icon: 'h-9 w-9 p-0',
  };
</script>

<button
  {type}
  {disabled}
  {title}
  {onclick}
  class={cn(
    'inline-flex shrink-0 items-center justify-center gap-2 rounded-md font-medium transition-colors',
    'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-zf-accent',
    'disabled:pointer-events-none disabled:opacity-50',
    variants[variant],
    sizes[size],
    className,
  )}
>
  {@render children?.()}
</button>
