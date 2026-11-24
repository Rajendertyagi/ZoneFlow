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
    default: 'bg-primary text-primary-foreground shadow-zf-1 hover:bg-primary-hover hover:shadow-zf-2',
    outline: 'border border-input bg-zf-surface text-foreground shadow-zf-1 hover:bg-secondary hover:shadow-zf-2',
    ghost: 'bg-transparent text-muted-foreground hover:bg-secondary hover:text-foreground',
    destructive: 'bg-destructive text-destructive-foreground shadow-zf-1 hover:brightness-110 hover:shadow-zf-2',
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
    'inline-flex shrink-0 items-center justify-center gap-2 rounded-md font-medium transition-colors motion-reduce:transition-none',
    'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary',
    'disabled:pointer-events-none disabled:opacity-50',
    variants[variant],
    sizes[size],
    className,
  )}
>
  {@render children?.()}
</button>
