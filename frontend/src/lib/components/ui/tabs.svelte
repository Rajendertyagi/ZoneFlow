<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  interface TabItem {
    value: string;
    label: string;
  }

  interface Props {
    items: TabItem[];
    value?: string;
    class?: string;
    children?: Snippet<[string]>;
  }

  let {
    items,
    value = $bindable(items[0]?.value ?? ''),
    class: className = '',
    children,
  }: Props = $props();
</script>

<div class={cn('flex min-h-0 flex-col', className)}>
  <div role="tablist" class="flex shrink-0 gap-1 border-b border-zf-border">
    {#each items as item (item.value)}
      <button
        type="button"
        role="tab"
        aria-selected={value === item.value}
        onclick={() => (value = item.value)}
        class={cn(
          'px-3 py-2 text-sm font-medium transition-colors',
          value === item.value
            ? 'border-b-2 border-zf-accent text-zf-accent'
            : 'text-zf-muted hover:text-zf-text',
        )}
      >
        {item.label}
      </button>
    {/each}
  </div>
  <div role="tabpanel" class="min-h-0 flex-1 pt-4">
    {@render children?.(value)}
  </div>
</div>
