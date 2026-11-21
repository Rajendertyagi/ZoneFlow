<script lang="ts">
  import { Home, Keyboard, LayoutDashboard, Settings2, X } from '@lucide/svelte';
  import { store } from '$lib/state/app.svelte';
  import { navigate, ui, type View } from '$lib/state/ui.svelte';
  import Badge from '$lib/components/ui/badge.svelte';

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

  const applied = $derived(store.snapshot?.applied || '—');
  const selected = $derived(store.snapshot?.selected || '—');
  const dirty = $derived(store.snapshot?.dirty ?? false);
  const enabled = $derived(store.snapshot?.enabled ?? false);
  const connectionVariant = $derived(
    store.connection === 'connected'
      ? 'success'
      : store.connection === 'lost'
        ? 'error'
        : 'warning',
  );
  const connectionLabel = $derived(
    store.connection === 'connected'
      ? 'connected'
      : store.connection === 'lost'
        ? 'connection lost'
        : store.connection === 'reconnecting'
          ? 'reconnecting'
          : 'connecting',
  );
</script>

<header class="flex flex-wrap items-center justify-between gap-3 border-b border-zf-border bg-zf-bg px-5 py-2.5">
  <div class="flex items-center gap-4">
    <h1 class="whitespace-nowrap text-sm font-semibold tracking-tight">
      ZoneFlow <span class="text-zf-accent">Studio</span>
    </h1>
    <nav class="flex items-center gap-0.5 rounded-md border border-zf-border bg-zf-surface p-0.5" aria-label="Main navigation">
      {#each items as item (item.view)}
        <button
          type="button"
          aria-current={ui.view === item.view ? 'page' : undefined}
          onclick={() => navigate(item.view)}
          class="inline-flex h-7 items-center gap-1.5 rounded px-2.5 text-xs font-medium transition-colors aria-[current=page]:bg-zf-accent aria-[current=page]:text-zf-accent-ink text-zf-muted hover:text-zf-text"
        >
          <item.icon size={13} /> {item.label}
        </button>
      {/each}
    </nav>
  </div>

  <div class="flex flex-wrap items-center gap-2 text-xs">
    <!-- Four independent state dimensions (D1–D6) -->
    <span class="flex items-center gap-1 rounded border border-zf-border bg-zf-surface px-2 py-1">
      <span class="text-[0.65rem] uppercase tracking-wide text-zf-faint">applied</span>
      <span class="font-medium {applied === '—' ? 'text-zf-faint' : 'text-zf-ok'}">{applied}</span>
    </span>
    <span class="flex items-center gap-1 rounded border border-zf-border bg-zf-surface px-2 py-1">
      <span class="text-[0.65rem] uppercase tracking-wide text-zf-faint">selected</span>
      <span class="font-medium text-zf-text">{selected}</span>
    </span>
    <span class="flex items-center gap-1 rounded border border-zf-border bg-zf-surface px-2 py-1">
      <span class="text-[0.65rem] uppercase tracking-wide text-zf-faint">saved</span>
      {#if dirty}
        <Badge variant="warning" class="!px-1.5 !py-0 !text-[0.6rem]">unsaved</Badge>
      {:else}
        <span class="font-medium text-zf-ok">saved</span>
      {/if}
    </span>
    <span class="flex items-center gap-1 rounded border border-zf-border bg-zf-surface px-2 py-1">
      <span class="text-[0.65rem] uppercase tracking-wide text-zf-faint">layout</span>
      <span class="font-medium {enabled ? 'text-zf-ok' : 'text-zf-error'}">{enabled ? 'enabled' : 'disabled'}</span>
    </span>
    <Badge variant={connectionVariant}>{connectionLabel}</Badge>
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
