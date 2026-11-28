<script lang="ts">
  import { Home, Keyboard, LayoutDashboard, Settings2, X } from '@lucide/svelte';
  import { navigate, ui, type View } from '$lib/state/ui.svelte';

  interface Props {
    onclose: () => void;
  }
  let { onclose }: Props = $props();

  const items: Array<{ view: View; label: string; icon: typeof Home }> = [
    { view: 'home', label: 'Home', icon: Home },
    { view: 'editor', label: 'Editor', icon: LayoutDashboard },
    { view: 'hotkeys', label: 'Hotkeys', icon: Keyboard },
    { view: 'settings', label: 'Settings', icon: Settings2 },
  ];
</script>

<header class="flex flex-wrap items-center justify-between gap-3 shadow-zf-topbar bg-zf-bg px-5 py-2.5">
  <div class="flex items-center gap-4">
    <h1 class="whitespace-nowrap text-base font-semibold tracking-tight">
      ZoneFlow <span class="text-zf-accent">Studio</span>
    </h1>
    <nav class="flex items-center gap-0.5 rounded-md bg-zf-surface p-0.5 shadow-zf-1" aria-label="Main navigation">
      {#each items as item (item.view)}
        <button
          type="button"
          aria-current={ui.view === item.view ? 'page' : undefined}
          onclick={() => navigate(item.view)}
          class="inline-flex h-7 items-center gap-1.5 rounded px-2.5 text-sm font-medium transition-colors aria-[current=page]:bg-zf-accent aria-[current=page]:text-zf-accent-ink text-zf-muted hover:text-zf-text"
        >
          <item.icon size={13} /> {item.label}
        </button>
      {/each}
    </nav>
  </div>

  <div class="flex flex-wrap items-center gap-2 text-xs">
    <button
      type="button"
      title="Close editor"
      aria-label="Close editor"
      class="rounded p-1.5 text-zf-faint hover:bg-zf-raised hover:text-zf-text"
      onclick={onclose}
    >
      <X size={15} />
    </button>
  </div>
</header>
