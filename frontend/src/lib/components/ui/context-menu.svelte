<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  interface MenuItem {
    label: string;
    onselect?: () => void;
    disabled?: boolean;
  }

  interface Props {
    items: MenuItem[];
    class?: string;
    children?: Snippet;
  }

  let { items, class: className = '', children }: Props = $props();

  let open = $state(false);
  let x = $state(0);
  let y = $state(0);

  function onContextMenu(event: MouseEvent): void {
    event.preventDefault();
    x = event.clientX;
    y = event.clientY;
    open = true;
  }

  function select(item: MenuItem): void {
    if (item.disabled) return;
    item.onselect?.();
    open = false;
  }
</script>

<svelte:window
  onclick={() => (open = false)}
  onkeydown={(event) => {
    if (event.key === 'Escape') open = false;
  }}
/>

<div role="presentation" class={cn('relative', className)} oncontextmenu={onContextMenu}>
  {@render children?.()}
</div>

{#if open}
  <div
    role="menu"
    class="fixed z-50 min-w-40 rounded-md border border-zf-border-strong bg-zf-panel p-1 shadow-xl"
    style:left="{x}px"
    style:top="{y}px"
  >
    {#each items as item (item.label)}
      <button
        type="button"
        role="menuitem"
        disabled={item.disabled}
        onclick={() => select(item)}
        class="block w-full rounded px-2 py-1.5 text-left text-sm text-zf-text hover:bg-zf-raised disabled:opacity-40"
      >
        {item.label}
      </button>
    {/each}
  </div>
{/if}
