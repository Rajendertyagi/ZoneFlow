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
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Switch from '$lib/components/ui/switch.svelte';
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

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <button
      type="button"
      aria-label="Close picker"
      class="absolute inset-0 cursor-default bg-scrim"
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

      <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-auto px-6 pt-4">
        <!-- Launcher script: primary selection for BAT users -->
        <div class="flex flex-col gap-1.5 rounded-md border border-zf-border bg-zf-surface p-3">
          <span class="text-xs font-medium text-zf-text">Launcher script (runs instead of the app)</span>
          <p class="text-[0.65rem] leading-relaxed text-zf-faint">
            Select a .bat/.cmd file. ZoneFlow executes this script; the executable below is only
            used to identify the window it opens. The script is never replaced by the EXE.
          </p>
          <div class="flex items-center gap-2">
            <Input
              value={launcherPath}
              placeholder="C:\Scripts\launch.bat"
              class="font-mono text-xs"
              onchange={(e) => (launcherPath = (e.currentTarget as HTMLInputElement).value)}
            />
            <Button size="sm" variant="outline" disabled={busy} onclick={() => void pickLauncher()}>
              <FolderOpen size={14} /> Select .bat/.cmd launcher…
            </Button>
          </div>
          {#if launcherError}
            <p class="text-[0.65rem] text-zf-error">
              Invalid launcher: {scriptPathErrorMessage(launcherError)}
            </p>
          {:else if launcherPath.trim()}
            <p class="text-[0.65rem] text-zf-ok">Valid launcher: {scriptDisplayName(launcherPath)}</p>
          {/if}
          <div class="flex items-center justify-between pt-1">
            <span class="text-xs text-zf-muted">Show console when running script</span>
            <Switch
              checked={showConsole}
              label="Show console when running script"
              onchange={(v) => (showConsole = v)}
            />
          </div>
        </div>

        <!-- Optional EXE override -->
        <div class="flex flex-col gap-1.5 rounded-md border border-zf-border bg-zf-surface p-3">
          <span class="text-xs font-medium text-zf-text">App to arrange (optional EXE override)</span>
          <p class="text-[0.65rem] leading-relaxed text-zf-faint">
            Leave empty to arrange the window the script opens. Set only if the script focuses an
            app that is already running. This EXE is never launched when a launcher is set.
          </p>
          <div class="flex items-center gap-2">
            <Input
              value={overridePath}
              placeholder="Full path to app.exe (optional)"
              class="font-mono text-xs"
              onchange={(e) => (overridePath = (e.currentTarget as HTMLInputElement).value)}
            />
          </div>
          <Input
            value={overrideExe}
            placeholder="app.exe (optional)"
            class="font-mono text-xs"
            onchange={(e) => (overrideExe = (e.currentTarget as HTMLInputElement).value)}
          />
        </div>

        <div class="flex flex-col gap-1.5">
          <span class="text-xs font-medium text-zf-text">Label</span>
          <Input
            value={label}
            placeholder="Display name for this zone"
            onchange={(e) => (label = (e.currentTarget as HTMLInputElement).value)}
          />
        </div>

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
                <div class="flex flex-col items-center justify-center gap-3 rounded-md border border-dashed border-zf-border bg-zf-surface px-4 py-6 text-center">
                  <FolderOpen size={24} class="text-zf-faint" />
                  <p class="text-xs text-zf-muted">Pick an executable to use as the optional override.</p>
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
          <Button size="sm" disabled={!canAssign} onclick={() => void assign()}>
            Assign to zone
          </Button>
        </div>
      </div>
    </div>
  </div>
{/if}
