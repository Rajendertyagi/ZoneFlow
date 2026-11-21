<script lang="ts">
  import { FolderOpen, GripVertical, RefreshCw, Search, Send } from '@lucide/svelte';
  import { api, type AppEntryDto } from '$lib/api/client';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage } from '$lib/state/workflow';
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Tabs from '$lib/components/ui/tabs.svelte';

  interface Props {
    /** Open the picker for the currently selected zone. */
    onpick: () => void;
    hasSelection: boolean;
  }
  let { onpick, hasSelection }: Props = $props();

  type Source = 'installed' | 'running';
  let source = $state<Source>('installed');
  let query = $state('');
  let apps = $state<AppEntryDto[]>([]);
  let loading = $state(false);

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
    loading = true;
    try {
      apps = await api.listApps(next);
    } catch (error) {
      toast('error', `Could not load ${next} apps: ${errorMessage(error)}`);
      apps = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const next = source;
    void load(next);
  });

  async function browse(): Promise<void> {
    loading = true;
    try {
      const picked = await api.browseApps();
      if (picked.length) {
        apps = picked;
        source = 'installed';
        toast('success', `Added ${picked.length} application(s) from disk.`);
      } else {
        toast('info', 'No executable was selected.');
      }
    } catch (error) {
      toast('error', `Browse failed: ${errorMessage(error)}`);
    } finally {
      loading = false;
    }
  }

  function onDragStart(event: DragEvent, app: AppEntryDto): void {
    const payload = JSON.stringify({ label: app.label, exe: app.exe, path: app.path });
    event.dataTransfer?.setData('application/x-zoneflow-app', payload);
    event.dataTransfer?.setData('text/plain', payload);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="flex items-center justify-between border-b border-zf-border px-3 py-2">
    <div>
      <h3 class="text-xs font-semibold uppercase tracking-wider text-zf-text">Applications</h3>
      <p class="text-[0.65rem] text-zf-faint">Reusable entries · drag onto a zone</p>
    </div>
    <Button
      variant="ghost"
      size="icon"
      title="Refresh list"
      disabled={loading}
      onclick={() => void load(source)}
    >
      <RefreshCw size={14} class={loading ? 'animate-spin' : ''} />
    </Button>
  </div>

  <div class="flex min-h-0 flex-1 flex-col px-3 pt-2">
    <Tabs
      bind:value={source}
      items={[
        { value: 'installed', label: 'Installed' },
        { value: 'running', label: 'Running' },
      ]}
      class="min-h-0 flex-1"
    >
      {#snippet children()}
        <div class="flex h-full min-h-0 flex-col gap-2">
          <div class="relative">
            <Search size={13} class="absolute left-2.5 top-2.5 text-zf-faint" />
            <Input bind:value={query} placeholder="Search apps…" class="h-8 pl-8 text-xs" />
          </div>

          <div class="min-h-0 flex-1 overflow-auto rounded-md border border-zf-border bg-zf-surface">
            {#if loading}
              <p class="p-3 text-xs text-zf-faint">Loading…</p>
            {:else if filtered.length === 0}
              <p class="p-3 text-xs text-zf-faint">No applications found.</p>
            {:else}
              {#each filtered as app (app.exe + app.path)}
                <div
                  draggable="true"
                  role="button"
                  tabindex="0"
                  title={`Drag ${app.label} onto a zone`}
                  ondragstart={(event) => onDragStart(event, app)}
                  class="group flex cursor-grab items-center gap-2 border-b border-zf-border/60 px-2 py-1.5 last:border-0 hover:bg-zf-raised active:cursor-grabbing"
                >
                  <GripVertical size={13} class="shrink-0 text-zf-faint" />
                  <span class="min-w-0 flex-1">
                    <span class="block truncate text-xs text-zf-text">{app.label}</span>
                    <span class="block truncate font-mono text-[0.65rem] text-zf-faint">{app.exe}</span>
                  </span>
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/snippet}
    </Tabs>
  </div>

  <div class="flex flex-col gap-2 border-t border-zf-border p-3">
    <Button variant="outline" size="sm" onclick={() => void browse()}>
      <FolderOpen size={14} /> Browse for .exe
    </Button>
    <Button variant="ghost" size="sm" disabled={!hasSelection} onclick={onpick} title={hasSelection ? '' : 'Select a zone first'}>
      <Send size={14} /> Assign to selected zone
    </Button>
  </div>
</div>
