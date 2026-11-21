<script lang="ts">
  import { Keyboard, RotateCcw, Save } from '@lucide/svelte';
  import { api } from '$lib/api/client';
  import { refreshState } from '$lib/api/events';
  import { store } from '$lib/state/app.svelte';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage } from '$lib/state/workflow';
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';

  const FRIENDLY: Record<string, string> = {
    clear_layout: 'Clear layout',
    move_left: 'Move window left',
    move_right: 'Move window right',
    move_up: 'Move window up',
    move_down: 'Move window down',
    maximize: 'Maximize window',
    restore: 'Restore window',
    reload: 'Reload configuration',
  };

  function label(name: string): string {
    if (FRIENDLY[name]) return FRIENDLY[name];
    const match = /^profile_(\d)$/.exec(name);
    return match ? `Apply profile ${match[1]}` : name;
  }

  let bindings = $state<Record<string, string>>({});
  let lastSnapshotHotkeys = '';

  $effect(() => {
    const rows = store.snapshot?.hotkeys ?? [];
    const signature = rows.map((r) => `${r.name}=${r.binding}`).join('|');
    if (signature !== lastSnapshotHotkeys) {
      lastSnapshotHotkeys = signature;
      bindings = Object.fromEntries(rows.map((r) => [r.name, r.binding]));
    }
  });

  const changed = $derived(
    (store.snapshot?.hotkeys ?? []).some((r) => bindings[r.name] !== r.binding),
  );

  async function save(): Promise<void> {
    try {
      await api.setHotkeys(bindings);
      await refreshState();
      toast('success', 'Hotkeys saved and re-registered globally.');
    } catch (error) {
      toast('error', `Could not save hotkeys: ${errorMessage(error)}`);
    }
  }

  function reset(): void {
    bindings = Object.fromEntries((store.snapshot?.hotkeys ?? []).map((r) => [r.name, r.binding]));
  }
</script>

<div class="h-full overflow-auto px-6 py-5">
  <div class="mx-auto max-w-3xl">
    <div class="mb-5 flex flex-wrap items-end justify-between gap-3">
      <div>
        <div class="flex items-center gap-2">
          <Keyboard size={18} class="text-zf-accent" />
          <h2 class="text-lg font-semibold text-zf-text">Hotkeys</h2>
        </div>
        <p class="mt-1 text-xs text-zf-muted">
          Global bindings work even when the editor is closed. Saving re-registers them immediately.
        </p>
      </div>
      <div class="flex gap-2">
        <Button variant="ghost" size="sm" disabled={!changed} onclick={reset}>
          <RotateCcw size={14} /> Reset
        </Button>
        <Button size="sm" disabled={!changed} onclick={() => void save()}>
          <Save size={14} /> Save hotkeys
        </Button>
      </div>
    </div>

    <div class="overflow-hidden rounded-lg border border-zf-border">
      {#each Object.keys(bindings).sort() as name (name)}
        <div class="flex items-center gap-4 border-b border-zf-border bg-zf-surface px-4 py-2.5 last:border-0">
          <span class="min-w-0 flex-1 text-sm text-zf-text">{label(name)}</span>
          <span class="hidden font-mono text-[0.65rem] text-zf-faint sm:block">{name}</span>
          <Input
            value={bindings[name]}
            class="h-8 w-52 font-mono text-xs"
            oninput={(e) => (bindings[name] = (e.currentTarget as HTMLInputElement).value)}
          />
        </div>
      {:else}
        <p class="px-4 py-6 text-center text-sm text-zf-faint">No hotkeys configured.</p>
      {/each}
    </div>

    <p class="mt-3 text-[0.7rem] leading-relaxed text-zf-faint">
      Use AHK-style modifiers: <span class="font-mono text-zf-muted">Ctrl</span>,
      <span class="font-mono text-zf-muted">Alt</span>,
      <span class="font-mono text-zf-muted">Shift</span>,
      <span class="font-mono text-zf-muted">Win</span>. Example:
      <span class="font-mono text-zf-muted">Ctrl+Shift+1</span>.
    </p>
  </div>
</div>
