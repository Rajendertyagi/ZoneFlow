<script lang="ts">
  import { FolderOpen, RefreshCw, Search } from '@lucide/svelte';
  import { api, type AppEntryDto, type ZoneDto } from '$lib/api/client';
  import { assignAppGuarded } from '$lib/actions';
  import { appsOnTabEnter } from '$lib/state/app-source';
  import {
    scriptDisplayName,
    scriptPathErrorMessage,
    validateScriptPath,
  } from '$lib/state/launcher';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage } from '$lib/state/workflow';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';

  const focusRing = 'focus-visible:outline-2 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-primary focus-visible:ring-0 motion-reduce:transition-none';

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

  // Launcher state. The script is the primary selection for BAT users; the
  // EXE override is optional and is NEVER launched instead of the script.
  let launcherPath = $state('');
  let showConsole = $state(true);
  let overrideExe = $state('');
  let overridePath = $state('');
  let label = $state('');

  const launcherError = $derived(
    launcherPath.trim() ? validateScriptPath(launcherPath) : null,
  );
  const canAssign = $derived(
    !busy && (launcherPath.trim().length > 0 || selected !== null) && !launcherError,
  );

  // Seed the form from the zone being edited whenever the dialog opens.
  $effect(() => {
    if (!open) return;
    launcherPath = zone?.launcher_path ?? '';
    showConsole = zone?.launcher_show_console ?? true;
    overrideExe = zone?.exe ?? '';
    overridePath = zone?.path ?? '';
    label = zone?.label ?? '';
    selected = null;
    query = '';
  });

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

  // Switching tabs loads that source. Browse is explicit (native file picker):
  // entering it clears any previously loaded list so a stale Installed/Running
  // result is never shown as if it were a browse result (B-6).
  $effect(() => {
    if (!open) return;
    const next = source;
    const browseApps = appsOnTabEnter(next);
    if (browseApps !== null) {
      apps = browseApps;
      selected = null;
      return;
    }
    void load(next);
  });

  async function pickLauncher(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const picked = await api.browseScripts();
      if (!picked.length) return;
      const path = picked[0].path;
      const error = validateScriptPath(path);
      if (error) {
        toast('error', `Invalid launcher: ${scriptPathErrorMessage(error)}`);
        return;
      }
      launcherPath = path;
      if (!label) label = scriptDisplayName(path);
      toast('success', `Launcher selected: ${scriptDisplayName(path)}`);
    } catch (error) {
      toast('error', `Could not select launcher: ${errorMessage(error)}`);
    } finally {
      busy = false;
    }
  }

  function chooseApp(app: AppEntryDto): void {
    selected = app;
    // Selecting an app fills the optional EXE override. It is used only to
    // identify the window; the launcher script still runs instead of it.
    overrideExe = app.exe;
    overridePath = app.path;
    if (!label) label = app.label;
  }

  async function assign(): Promise<void> {
    if (!zone || !canAssign) return;
    busy = true;
    const ok = await assignAppGuarded(zone, {
      label: label || scriptDisplayName(launcherPath) || overrideExe,
      exe: overrideExe,
      path: overridePath,
      launcher_path: launcherPath.trim(),
      launcher_show_console: showConsole,
    });
    busy = false;
    if (ok) onclose();
  }
</script>

<Dialog.Root
  open={open}
  onOpenChange={(next) => {
    if (!next) onclose();
  }}
>
  <Dialog.Content
    showCloseButton={false}
    aria-label="Assign application"
    class="flex max-h-[85vh] max-w-2xl flex-col gap-0 overflow-hidden rounded-lg border border-zf-border-strong bg-zf-panel p-0 shadow-2xl ring-0 sm:max-w-2xl"
  >
    <Dialog.Header class="flex flex-row items-center justify-between gap-0 border-b border-zf-border px-6 py-4">
      <div class="min-w-0">
        <Dialog.Title class="text-base font-semibold text-zf-text">Assign an application</Dialog.Title>
        <Dialog.Description class="text-xs text-zf-muted">
          {zone ? `Target: zone ${zone.id}${zone.label ? ` · ${zone.label}` : ''}` : 'No zone selected'}
        </Dialog.Description>
      </div>
      <Dialog.Close>
        {#snippet child({ props })}
          <Button variant="ghost" size="icon" title="Close" aria-label="Close" class={`h-9 w-9 p-0 ${focusRing}`} {...props}>×</Button>
        {/snippet}
      </Dialog.Close>
    </Dialog.Header>

      <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-auto px-6 pt-4">
        <!-- Launcher script: primary selection for BAT users -->
        <div class="flex flex-col gap-1.5 rounded-md border border-zf-border bg-zf-surface p-3">
          <span class="text-xs font-medium text-zf-text">Launcher script (runs instead of the app)</span>
          <p class="text-xs leading-relaxed text-zf-faint">
            Select a .bat/.cmd file. ZoneFlow executes this script; the executable below is only
            used to identify the window it opens. The script is never replaced by the EXE.
          </p>
          <div class="flex items-center gap-2">
            <Input
              bind:value={launcherPath}
              placeholder="C:\Scripts\launch.bat"
              class="border border-border focus-visible:border-primary font-mono text-xs"
            />
            <Button size="sm" variant="outline" disabled={busy} class={`h-7 text-xs ${focusRing}`} onclick={() => void pickLauncher()}>
              <FolderOpen size={14} /> Select .bat/.cmd launcher…
            </Button>
          </div>
          {#if launcherError}
            <p class="text-xs text-zf-error">
              Invalid launcher: {scriptPathErrorMessage(launcherError)}
            </p>
          {:else if launcherPath.trim()}
            <p class="text-xs text-zf-ok">Valid launcher: {scriptDisplayName(launcherPath)}</p>
          {/if}
          <div class="flex items-center justify-between pt-1">
            <span class="text-xs text-zf-muted">Show console when running script</span>
            <Switch
              bind:checked={showConsole}
              aria-label="Show console when running script"
              class={focusRing}
            />
          </div>
        </div>

        <!-- Optional EXE override -->
        <div class="flex flex-col gap-1.5 rounded-md border border-zf-border bg-zf-surface p-3">
          <span class="text-xs font-medium text-zf-text">App to arrange (optional EXE override)</span>
          <p class="text-xs leading-relaxed text-zf-faint">
            Leave empty to arrange the window the script opens. Set only if the script focuses an
            app that is already running. This EXE is never launched when a launcher is set.
          </p>
          <div class="flex items-center gap-2">
            <Input
              bind:value={overridePath}
              placeholder="Full path to app.exe (optional)"
              class="border border-border focus-visible:border-primary font-mono text-xs"
            />
          </div>
          <Input
            bind:value={overrideExe}
            placeholder="app.exe (optional)"
            class="border border-border focus-visible:border-primary font-mono text-xs"
          />
        </div>

        <div class="flex flex-col gap-1.5">
          <span class="text-xs font-medium text-zf-text">Label</span>
          <Input
            bind:value={label}
            placeholder="Display name for this zone"
            class="border border-border focus-visible:border-primary"
          />
        </div>

        {#snippet panel(value: Source)}
          <div class="flex h-full min-h-0 flex-col gap-3">
            <div class="flex items-center gap-2">
              <div class="relative flex-1">
                <Search size={14} class="absolute left-2.5 top-2.5 text-zf-faint" />
                <Input bind:value={query} placeholder="Search applications…" class="border border-border focus-visible:border-primary pl-8" />
              </div>
              {#if value !== 'browse'}
                <Button
                  variant="outline"
                  size="icon"
                  title="Refresh list"
                  disabled={loading}
                  class={`h-9 w-9 p-0 ${focusRing}`}
                  onclick={() => void load(value as Source)}
                >
                  <RefreshCw size={14} class={loading ? 'animate-spin' : ''} />
                </Button>
              {/if}
            </div>

            {#if value === 'browse'}
              <div class="flex flex-col items-center justify-center gap-3 rounded-md border border-dashed border-zf-border bg-zf-surface px-4 py-6 text-center">
                <FolderOpen size={24} class="text-zf-faint" />
                <p class="text-xs text-zf-muted">Pick an executable to use as the optional override.</p>
                <Button size="sm" variant="outline" class={`h-7 text-xs ${focusRing}`} onclick={() => void load('browse')}>
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
                {#each filtered as app (app.id)}
                  <button
                    type="button"
                    onclick={() => chooseApp(app)}
                    class="flex w-full items-center gap-3 border-b border-zf-border/60 px-3 py-2 text-left last:border-0 hover:bg-zf-raised {selected === app
                      ? 'bg-zf-accent/10'
                      : ''}"
                  >
                    <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded bg-zf-raised text-xs font-semibold text-zf-accent">
                      {app.label.slice(0, 1).toUpperCase()}
                    </span>
                    <span class="min-w-0 flex-1">
                      <span class="block truncate text-sm text-zf-text">{app.label}</span>
                      <span class="block truncate font-mono text-xs text-zf-faint">
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

        <Tabs.Root bind:value={source} class="flex min-h-0 flex-1 flex-col">
          <Tabs.List aria-label="Application sources" class="flex w-full shrink-0 gap-1 border-b border-border bg-transparent p-0">
            <Tabs.Trigger value="installed" class={`px-3 py-2 ${focusRing}`}>Installed</Tabs.Trigger>
            <Tabs.Trigger value="running" class={`px-3 py-2 ${focusRing}`}>Running</Tabs.Trigger>
            <Tabs.Trigger value="browse" class={`px-3 py-2 ${focusRing}`}>Browse…</Tabs.Trigger>
          </Tabs.List>
          <Tabs.Content value="installed" tabindex={-1} class="min-h-0 flex-1 pt-4">{@render panel('installed')}</Tabs.Content>
          <Tabs.Content value="running" tabindex={-1} class="min-h-0 flex-1 pt-4">{@render panel('running')}</Tabs.Content>
          <Tabs.Content value="browse" tabindex={-1} class="min-h-0 flex-1 pt-4">{@render panel('browse')}</Tabs.Content>
        </Tabs.Root>
      </div>

    <Dialog.Footer
      class="mx-0 mb-0 flex flex-row items-center justify-between gap-2 rounded-none border-zf-border bg-transparent p-0 px-6 py-4 sm:justify-between"
    >
      <p class="text-xs text-zf-faint">Library entries are reusable — assigning never removes them.</p>
      <div class="flex gap-2">
        <Button variant="ghost" size="sm" class={`h-7 text-xs ${focusRing}`} onclick={onclose}>Cancel</Button>
        <Button size="sm" disabled={!canAssign} class={`h-9 px-4 ${focusRing}`} onclick={() => void assign()}>
          Assign to zone
        </Button>
      </div>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
