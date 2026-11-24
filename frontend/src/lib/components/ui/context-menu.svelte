<script lang="ts">
  import { tick } from 'svelte';
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';
  import { edgeEnabledIndex, enabledIndices, nextEnabledIndex } from './a11y-helpers';

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
  let activeIndex = $state(-1);
  let triggerEl: HTMLDivElement | null = $state(null);
  let menuEl: HTMLDivElement | null = $state(null);

  // `tick()` waits for the {#if open} block to mount and `menuEl` to be bound,
  // so focus always lands on a real menuitem instead of racing the DOM update
  // with a microtask.
  async function focusItem(index: number): Promise<void> {
    activeIndex = index;
    await tick();
    menuEl?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')[index]?.focus();
  }

  function openMenu(clientX: number, clientY: number): void {
    x = clientX;
    y = clientY;
    open = true;
    void focusItem(enabledIndices(items)[0] ?? -1);
  }

  async function closeMenu(restoreFocus = true): Promise<void> {
    open = false;
    activeIndex = -1;
    if (restoreFocus) {
      await tick();
      triggerEl?.focus();
    }
  }

  function onContextMenu(event: MouseEvent): void {
    event.preventDefault();
    openMenu(event.clientX, event.clientY);
  }

  function onTriggerKeydown(event: KeyboardEvent): void {
    // Standard keyboard equivalents of a right-click.
    if (event.key !== 'ContextMenu' && !(event.shiftKey && event.key === 'F10')) return;
    event.preventDefault();
    const rect = triggerEl?.getBoundingClientRect();
    openMenu(rect ? rect.left + 8 : 0, rect ? rect.top + 8 : 0);
  }

  function select(item: MenuItem): void {
    if (item.disabled) return;
    item.onselect?.();
    void closeMenu();
  }

  function onMenuKeydown(event: KeyboardEvent): void {
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        void focusItem(nextEnabledIndex(items, activeIndex, true));
        break;
      case 'ArrowUp':
        event.preventDefault();
        void focusItem(nextEnabledIndex(items, activeIndex, false));
        break;
      case 'Home':
        event.preventDefault();
        void focusItem(edgeEnabledIndex(items, 'home'));
        break;
      case 'End':
        event.preventDefault();
        void focusItem(edgeEnabledIndex(items, 'end'));
        break;
      case 'Enter':
      case ' ':
        event.preventDefault();
        if (activeIndex >= 0) select(items[activeIndex]);
        break;
      case 'Escape':
        event.preventDefault();
        void closeMenu();
        break;
      case 'Tab':
        // Unlike Escape, Tab must NOT pull focus back to the trigger: that
        // would create a two-element focus loop and break the page's tab
        // sequence. Close without restoring and let the browser move focus.
        void closeMenu(false);
        break;
    }
  }
</script>

<svelte:window
  onclick={() => {
    if (open) void closeMenu(false);
  }}
  onkeydown={(event) => {
    if (!open || event.key !== 'Escape') return;
    // The menu-level handler owns Escape while focus is inside the menu. This
    // is only a fallback for when focus never entered it (e.g. all items
    // disabled), so Escape closes and restores focus exactly once.
    if (menuEl?.contains(document.activeElement as Node | null)) return;
    void closeMenu();
  }}
/>

<!-- The wrapper is focusable so keyboard users can open the menu with the
     ContextMenu key / Shift+F10. The focus-visible ring keeps that extra tab
     stop perceivable for keyboard users. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  bind:this={triggerEl}
  role="presentation"
  tabindex="0"
  class={cn(
    'relative focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary',
    className,
  )}
  oncontextmenu={onContextMenu}
  onkeydown={onTriggerKeydown}
>
  {@render children?.()}
</div>

{#if open}
  <div
    bind:this={menuEl}
    role="menu"
    aria-label="Context menu"
    tabindex="-1"
    class="fixed z-50 min-w-40 rounded-md border border-input bg-popover p-1 shadow-zf-3 focus:outline-none"
    style:left="{x}px"
    style:top="{y}px"
    onkeydown={onMenuKeydown}
  >
    {#each items as item, index (item.label)}
      <button
        type="button"
        role="menuitem"
        disabled={item.disabled}
        tabindex={index === activeIndex ? 0 : -1}
        onclick={() => select(item)}
        class="block w-full rounded px-2 py-1.5 text-left text-sm text-foreground hover:bg-secondary focus-visible:bg-secondary focus-visible:outline-2 focus-visible:outline-primary disabled:opacity-40"
      >
        {item.label}
      </button>
    {/each}
  </div>
{/if}
