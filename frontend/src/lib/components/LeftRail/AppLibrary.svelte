<script lang="ts">
  import { FolderOpen, GripVertical, RefreshCw, Search, Send } from '@lucide/svelte';
  import { api, type AppEntryDto } from '$lib/api/client';
  import { sourceAfterBrowse } from '$lib/state/app-source';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage } from '$lib/state/workflow';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';

  const focusRing = 'focus-visible:outline-2 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-primary focus-visible:ring-0 motion-reduce:transition-none';

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
        // B-3: keep the active tab. Switching `source` here would re-run the
        // load effect and immediately overwrite the picked entries.
        source = sourceAfterBrowse(source);
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
    // A complete AssignDto: no launcher, console default on. The backend also
    // defaults these fields, but sending them explicitly keeps the wire shape
    // consistent with the picker.
    const payload = JSON.stringify({
      label: app.label,
      exe: app.exe,
      path: app.path,
      launcher_path: '',
      launcher_show_console: true,
    });
    event.dataTransfer?.setData('application/x-zoneflow-app', payload);
    event.dataTransfer?.setData('text/plain', payload);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="flex items-center justify-between border-b border-zf-border px-3 py-2">
    <div>
      <h3 class="text-xs font-semibold uppercase tracking-wider text-zf-text">Applications</h3>
      <p class="text-xs text-zf-muted">Reusable entries · drag onto a zone</p>
    </div>
    <Button
      variant="ghost"
      size="icon"
      title="Refresh list"
      disabled={loading}
      class={`h-9 w-9 p-0 ${focusRing}`}
      onclick={() => void load(source)}
    >
      <RefreshCw size={14} class={loading ? 'animate-spin' : ''} />
    </Button>
  </div>

  <div class="flex min-h-0 flex-1 flex-col px-3 pt-2">
    {#snippet list()}
      <div class="min-h-0 flex-1 overflow-y-auto rounded-md">
        {#if loading}
          <p class="p-3 text-sm text-zf-faint">Loading…</p>
        {:else if filtered.length === 0}
          <p class="p-3 text-sm text-zf-faint">No applications found.</p>
        {:else}
          {#each filtered as app (app.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              draggable="true"
              title={`Drag ${app.label} onto a zone`}
              ondragstart={(event) => onDragStart(event, app)}
              class={`group flex min-h-9 cursor-grab items-center gap-3 border-b border-[color:var(--zf-hairline)] px-3 py-2 transition-colors last:border-0 hover:bg-zf-raised active:cursor-grabbing ${focusRing}`}
            >
              <GripVertical size={16} class="shrink-0 text-zf-faint" />
              <span class="min-w-0 flex-1">
                <span class="block line-clamp-2 break-words text-sm font-normal leading-snug text-zf-text">{app.label}</span>
                <span class="block truncate font-mono text-xs leading-snug text-zf-faint">{app.exe}</span>
              </span>
            </div>
          {/each}
        {/if}
      </div>
    {/snippet}

    <Tabs.Root bind:value={source} class="flex min-h-0 flex-1 flex-col">
      <Tabs.List aria-label="Library sections" class="flex w-full shrink-0 gap-1 border-b border-border bg-transparent p-0">
        <Tabs.Trigger value="installed" class={`px-3 py-2 ${focusRing}`}>Installed</Tabs.Trigger>
        <Tabs.Trigger value="running" class={`px-3 py-2 ${focusRing}`}>Running</Tabs.Trigger>
      </Tabs.List>

      <!-- P4: the section header and search stay pinned; only the list scrolls. -->
      <div class="relative my-2 shrink-0">
        <Search size={13} class="absolute left-2.5 top-2.5 text-zf-faint" />
        <Input bind:value={query} placeholder="Search apps…" class="h-8 border border-border pl-8 text-xs focus-visible:border-primary" />
      </div>

      <Tabs.Content value="installed" tabindex={-1} class="flex min-h-0 flex-1 flex-col">{@render list()}</Tabs.Content>
      <Tabs.Content value="running" tabindex={-1} class="flex min-h-0 flex-1 flex-col">{@render list()}</Tabs.Content>
    </Tabs.Root>
  </div>

  <div class="flex flex-col gap-2 border-t border-zf-border p-3">
    <Button variant="outline" size="sm" class={`h-7 text-xs ${focusRing}`} onclick={() => void browse()}>
      <FolderOpen size={14} /> Browse for .exe
    </Button>
    <Button variant="ghost" size="sm" disabled={!hasSelection} class={`h-7 text-xs ${focusRing}`} onclick={onpick} title={hasSelection ? '' : 'Select a zone first'}>
      <Send size={14} /> Assign to selected zone
    </Button>
  </div>
</div>
