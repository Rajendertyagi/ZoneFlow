<script lang="ts">
  import { FolderOpen, RefreshCw, Search } from '@lucide/svelte';
  import { api, type AppEntryDto, type ZoneDto } from '$lib/api/client';
  import { assignAppGuarded } from '$lib/actions';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage } from '$lib/state/workflow';
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Tabs from '$lib/components/ui/tabs.svelte';

  interface Props {
    open: boolean;
    zone: ZoneDto | null;
    onclose: () => void;
  }
  let { open, zone, onclose }: Props = $props();

  type Source = 'installed' | 'running' | 'browse';
  let source = $state<Source>('installed');
  let query = $state('');
  let apps = $state<AppEntryDto[]>([]);
  let selected = $state<AppEntryDto | null>(null);
  let loading = $state(false);
  let busy = $state(false);

  const filtered = $derived(
    apps.filter((app) => {
      const q = query.trim().toLowerCase();
      if (!q) return true;
      return (
        app.label.toLowerCase().includes(q) ||
        app.exe.toLowerCase().includes(q) ||
        app.path.toLowerCase().includes(q)
      );
    }),
  );

  async function load(next: Source): Promise<void> {
    selected = null;
    loading = true;
    try {
      if (next === 'browse') {
        const picked = await api.browseApps();
        apps = picked;
        toast(picked.length ? 'success' : 'info', `Browse returned ${picked.length} application(s).`);
      } else {
        apps = await api.listApps(next);
      }
    } catch (error) {
      toast('error', `Could not load apps: ${errorMessage(error)}`);
      apps = [];
    } finally {
      loading = false;
    }
  }

  // Switching tabs loads that source. Browse is explicit (native file picker).
  $effect(() => {
    if (!open) return;
    const next = source;
    if (next === 'browse') return;
    void load(next);
  });

  async function assign(): Promise<void> {
    if (!zone || !selected || busy) return;
    busy = true;
    const ok = await assignAppGuarded(zone, {
      label: selected.label,
      exe: selected.exe,
      path: selected.path,
    });
    busy = false;
    if (ok) onclose();
  }
</script>

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <button
      type="button"
      aria-label="Close picker"
      class="absolute inset-0 cursor-default bg-black/70"
      onclick={onclose}
    ></button>
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Assign application"
      class="relative z-10 flex max-h-[85vh] w-full max-w-2xl flex-col overflow-hidden rounded-lg border border-zf-border-strong bg-zf-panel shadow-2xl"
    >
      <div class="flex items-center justify-between border-b border-zf-border px-6 py-4">
        <div>
          <h2 class="text-base font-semibold text-zf-text">Assign an application</h2>
          <p class="text-xs text-zf-muted">
            {zone ? `Target: zone ${zone.id}${zone.label ? ` · ${zone.label}` : ''}` : 'No zone selected'}
          </p>
        </div>
        <Button variant="ghost" size="icon" title="Close" onclick={onclose}>×</Button>
      </div>

      <div class="flex min-h-0 flex-1 flex-col px-6 pt-4">
        <Tabs
          bind:value={source}
          items={[
            { value: 'installed', label: 'Installed' },
            { value: 'running', label: 'Running' },
            { value: 'browse', label: 'Browse…' },
          ]}
          class="min-h-0 flex-1"
        >
          {#snippet children(value)}
            <div class="flex h-full min-h-0 flex-col gap-3">
              <div class="flex items-center gap-2">
                <div class="relative flex-1">
                  <Search size={14} class="absolute left-2.5 top-2.5 text-zf-faint" />
                  <Input bind:value={query} placeholder="Search applications…" class="pl-8" />
                </div>
                {#if value !== 'browse'}
                  <Button
                    variant="outline"
                    size="icon"
                    title="Refresh list"
                    disabled={loading}
                    onclick={() => void load(value as Source)}
                  >
                    <RefreshCw size={14} class={loading ? 'animate-spin' : ''} />
                  </Button>
                {/if}
              </div>

              {#if value === 'browse'}
                <div class="flex flex-col items-center justify-center gap-3 rounded-md border border-dashed border-zf-border bg-zf-surface px-4 py-10 text-center">
                  <FolderOpen size={28} class="text-zf-faint" />
                  <p class="text-sm text-zf-muted">Pick a custom executable from disk.</p>
                  <Button size="sm" variant="outline" onclick={() => void load('browse')}>
                    Open file picker
                  </Button>
                </div>
              {/if}

              <div class="min-h-0 flex-1 overflow-auto rounded-md border border-zf-border bg-zf-surface">
                {#if loading}
                  <p class="p-4 text-sm text-zf-faint">Loading…</p>
                {:else if filtered.length === 0}
                  <p class="p-4 text-sm text-zf-faint">No applications found.</p>
                {:else}
                  {#each filtered as app (app.exe + app.path)}
                    <button
                      type="button"
                      onclick={() => (selected = app)}
                      class="flex w-full items-center gap-3 border-b border-zf-border/60 px-3 py-2 text-left last:border-0 hover:bg-zf-raised {selected === app
                        ? 'bg-zf-accent/10'
                        : ''}"
                    >
                      <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded bg-zf-raised text-xs font-semibold text-zf-accent">
                        {app.label.slice(0, 1).toUpperCase()}
                      </span>
                      <span class="min-w-0 flex-1">
                        <span class="block truncate text-sm text-zf-text">{app.label}</span>
                        <span class="block truncate font-mono text-[0.7rem] text-zf-faint">
                          {app.exe}{app.path ? ` · ${app.path}` : ''}
                        </span>
                      </span>
                      {#if selected === app}
                        <span class="shrink-0 text-xs font-medium text-zf-accent">selected</span>
                      {/if}
                    </button>
                  {/each}
                {/if}
              </div>
            </div>
          {/snippet}
        </Tabs>
      </div>

      <div class="flex items-center justify-between gap-2 border-t border-zf-border px-6 py-4">
        <p class="text-[0.7rem] text-zf-faint">Library entries are reusable — assigning never removes them.</p>
        <div class="flex gap-2">
          <Button variant="ghost" size="sm" onclick={onclose}>Cancel</Button>
          <Button size="sm" disabled={!zone || !selected || busy} onclick={() => void assign()}>
            Assign to zone
          </Button>
        </div>
      </div>
    </div>
  </div>
{/if}
