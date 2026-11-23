<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  interface Props {
    class?: string;
    children?: Snippet;
  }

  let { class: className = '', children }: Props = $props();
</script>

<div
  class={cn(
    'overflow-auto [scrollbar-color:var(--color-input)_transparent] [scrollbar-width:thin]',
    className,
  )}
>
  {@render children?.()}
</div>
