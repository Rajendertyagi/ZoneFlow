<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  interface Props {
    text: string;
    class?: string;
    children?: Snippet;
  }

  let { text, class: className = '', children }: Props = $props();
  let visible = $state(false);
</script>

<span
  role="presentation"
  class={cn('relative inline-flex', className)}
  onmouseenter={() => (visible = true)}
  onmouseleave={() => (visible = false)}
>
  {@render children?.()}
  {#if visible}
    <span
      role="tooltip"
      class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-1 -translate-x-1/2 whitespace-nowrap rounded border border-zf-border-strong bg-zf-panel px-2 py-1 text-xs text-zf-text shadow-lg"
    >
      {text}
    </span>
  {/if}
</span>
