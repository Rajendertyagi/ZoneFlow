<script lang="ts">
  import { tick } from 'svelte';
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';
  import { edgeIndex, rovingStep } from './a11y-helpers';

  interface TabItem {
    value: string;
    label: string;
  }

  interface Props {
    items: TabItem[];
    value?: string;
    class?: string;
    /** Optional accessible name for the tablist. */
    label?: string;
    children?: Snippet<[string]>;
  }

  let {
    items,
    value = $bindable(items[0]?.value ?? ''),
    class: className = '',
    label,
    children,
  }: Props = $props();

  const uid = $props.id();
  const baseId = `zf-tabs-${uid}`;
  const panelId = `${baseId}-panel`;
  const tabId = (itemValue: string) => `${baseId}-tab-${itemValue}`;

  let tablistEl: HTMLDivElement | null = $state(null);

  async function activate(index: number): Promise<void> {
    const item = items[index];
    if (!item) return;
    value = item.value;
    // Wait for the roving-tabindex update before focusing the new tab.
    await tick();
    tablistEl?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[index]?.focus();
  }

  function onTablistKeydown(event: KeyboardEvent): void {
    const current = items.findIndex((item) => item.value === value);
    if (current < 0) return;
    let next = -1;
    switch (event.key) {
      case 'ArrowRight':
      case 'ArrowDown':
        next = rovingStep(current, items.length, true);
        break;
      case 'ArrowLeft':
      case 'ArrowUp':
        next = rovingStep(current, items.length, false);
        break;
      case 'Home':
        next = edgeIndex('home', items.length);
        break;
      case 'End':
        next = edgeIndex('end', items.length);
        break;
      default:
        return;
    }
    event.preventDefault();
    void activate(next);
  }
</script>

<div class={cn('flex min-h-0 flex-col', className)}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    bind:this={tablistEl}
    role="tablist"
    aria-label={label}
    tabindex="-1"
    class="flex shrink-0 gap-1 border-b border-border"
    onkeydown={onTablistKeydown}
  >
    {#each items as item (item.value)}
      <button
        type="button"
        role="tab"
        id={tabId(item.value)}
        aria-selected={value === item.value}
        aria-controls={panelId}
        tabindex={value === item.value ? 0 : -1}
        onclick={() => (value = item.value)}
        class={cn(
          'px-3 py-2 text-sm font-medium transition-colors motion-reduce:transition-none',
          'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary',
          value === item.value
            ? 'border-b-2 border-primary text-primary'
            : 'text-muted-foreground hover:text-foreground',
        )}
      >
        {item.label}
      </button>
    {/each}
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    id={panelId}
    role="tabpanel"
    aria-labelledby={value ? tabId(value) : undefined}
    tabindex="0"
    class="min-h-0 flex-1 pt-4 focus:outline-none"
  >
    {@render children?.(value)}
  </div>
</div>
