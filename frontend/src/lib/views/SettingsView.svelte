<script lang="ts">
  import { RotateCcw, Save, Settings2 } from '@lucide/svelte';
  import { api, type AppOptionsDto, type CloseAction, type SettingsDto } from '$lib/api/client';
  import { refreshState } from '$lib/api/events';
  import { store } from '$lib/state/app.svelte';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage } from '$lib/state/workflow';
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import Switch from '$lib/components/ui/switch.svelte';

  // Non-nullable so event handlers stay type-safe; `loaded` gates rendering
  // until the authoritative snapshot has seeded these values.
  const PLACEHOLDER: SettingsDto = {
    snap_points: [25, 50, 75],
    grid_cols: 12,
    grid_rows: 8,
    min_zone_pct: 15,
    resize_step_pct: 1,
    boundary_tolerance_pct: 1.5,
    undo_depth: 60,
    display_debounce_ms: 200,
    overlay_inactive_alpha: 200,
    overlay_active_alpha: 255,
    dwm_snap_retries: 2,
    min_window_px: 150,
    zone_tints: ['#F5B544', '#6EA8FE', '#5BD6A0', '#C792EA', '#FF8A65', '#4DD0E1'],
    auto_launch_on_startup: false,
    resolutions: [],
    app_list_cap: 300,
    running_apps_ttl_secs: 2,
    taskbar_h: 0,
    stage_lower_h: 0,
  };

  let settings = $state<SettingsDto>({ ...PLACEHOLDER });
  let options = $state<AppOptionsDto>({ auto_start: false, close_action: 'ask', gap: 8 });
  let snapText = $state('25, 50, 75');
  let loaded = $state(false);
  let signature = '';
  let busy = $state(false);

  $effect(() => {
    const snapshot = store.snapshot;
    if (!snapshot) return;
    const nextSignature = JSON.stringify(snapshot.settings) + JSON.stringify(snapshot.draft.auto_start) +
      snapshot.draft.close_action + snapshot.draft.gap;
    if (nextSignature === signature) return;
    signature = nextSignature;
    settings = { ...snapshot.settings };
    loaded = true;
    options = {
      auto_start: snapshot.draft.auto_start,
      close_action: snapshot.draft.close_action,
      gap: snapshot.draft.gap,
    };
    snapText = snapshot.settings.snap_points.join(', ');
  });

  function num(value: string, fallback: number): number {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  async function save(): Promise<void> {
    if (!loaded || busy) return;
    busy = true;
    try {
      const snapPoints = snapText
        .split(',')
        .map((s) => Number(s.trim()))
        .filter((n) => Number.isFinite(n) && n >= 0 && n <= 100);
      await api.setSettings({ ...settings, snap_points: snapPoints });
      await api.setAppOptions(options);
      await refreshState();
      toast('success', 'Settings applied to draft — save the profile to persist.');
    } catch (error) {
      toast('error', `Could not save settings: ${errorMessage(error)}`);
      await refreshState();
    } finally {
      busy = false;
    }
  }

  function reset(): void {
    signature = '';
  }
</script>

<div class="h-full overflow-auto px-6 py-5">
  <div class="mx-auto flex max-w-4xl flex-col gap-5">
    <div class="flex flex-wrap items-end justify-between gap-3">
      <div>
        <div class="flex items-center gap-2">
          <Settings2 size={18} class="text-zf-accent" />
          <h2 class="text-lg font-semibold text-zf-text">Settings</h2>
        </div>
        <p class="mt-1 text-xs text-zf-muted">
          Editor behavior, overlay, startup and close handling. Changes are applied through the backend.
        </p>
      </div>
      <div class="flex gap-2">
        <Button variant="ghost" size="sm" onclick={reset}>
          <RotateCcw size={14} /> Reload
        </Button>
        <Button size="sm" disabled={!loaded || busy} onclick={() => void save()}>
          <Save size={14} /> Apply settings
        </Button>
      </div>
    </div>

    <p class="text-[0.7rem] leading-relaxed text-zf-muted">
      Settings are part of your draft. Use Save in the editor to persist them to disk.
    </p>

    {#if !loaded}
      <p class="text-sm text-zf-faint">Loading settings…</p>
    {:else}
      <div class="grid gap-5 lg:grid-cols-2">
        <!-- Editor -->
        <section class="rounded-lg border border-zf-border bg-zf-surface p-4">
          <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-zf-accent">Editor</h3>
          <div class="flex flex-col gap-3">
            <div class="grid grid-cols-2 gap-3">
              <div class="flex flex-col gap-1.5">
                <Label for="set-cols">Grid columns</Label>
                <input id="set-cols" type="number" min="1" max="24" value={settings.grid_cols}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.grid_cols = num((e.currentTarget as HTMLInputElement).value, settings.grid_cols))} />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-rows">Grid rows</Label>
                <input id="set-rows" type="number" min="1" max="24" value={settings.grid_rows}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.grid_rows = num((e.currentTarget as HTMLInputElement).value, settings.grid_rows))} />
              </div>
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-snap">Snap points (% · comma separated)</Label>
              <Input id="set-snap" bind:value={snapText} class="font-mono text-xs" />
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div class="flex flex-col gap-1.5">
                <Label for="set-min">Min zone %</Label>
                <input id="set-min" type="number" min="1" max="50" step="0.5" value={settings.min_zone_pct}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.min_zone_pct = num((e.currentTarget as HTMLInputElement).value, settings.min_zone_pct))} />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-gap">Zone gap %</Label>
                <input id="set-gap" type="number" min="0" max="20" value={options.gap}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (options.gap = num((e.currentTarget as HTMLInputElement).value, options.gap))} />
              </div>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div class="flex flex-col gap-1.5">
                <Label for="set-tol">Boundary tolerance %</Label>
                <input id="set-tol" type="number" min="0" max="10" step="0.1" value={settings.boundary_tolerance_pct}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.boundary_tolerance_pct = num((e.currentTarget as HTMLInputElement).value, settings.boundary_tolerance_pct))} />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-step">Resize step %</Label>
                <input id="set-step" type="number" min="0.5" max="25" step="0.5" value={settings.resize_step_pct}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.resize_step_pct = num((e.currentTarget as HTMLInputElement).value, settings.resize_step_pct))} />
              </div>
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-undo">Undo depth</Label>
              <input id="set-undo" type="number" min="1" max="200" value={settings.undo_depth}
                class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                onchange={(e) => (settings.undo_depth = num((e.currentTarget as HTMLInputElement).value, settings.undo_depth))} />
            </div>
          </div>
        </section>

        <!-- Startup & close -->
        <section class="rounded-lg border border-zf-border bg-zf-surface p-4">
          <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-zf-accent">Startup &amp; close</h3>
          <div class="flex flex-col gap-3">
            <div class="flex items-center justify-between rounded-md border border-zf-border bg-zf-bg px-3 py-2">
              <div>
                <p class="text-xs font-medium text-zf-text">Start with Windows</p>
                <p class="text-[0.65rem] text-zf-faint">Registers ZoneFlow in the user Run key.</p>
              </div>
              <Switch checked={options.auto_start} label="Start with Windows" onchange={(v) => (options.auto_start = v)} />
            </div>
            <div class="flex items-center justify-between rounded-md border border-zf-border bg-zf-bg px-3 py-2">
              <div>
                <p class="text-xs font-medium text-zf-text">Launch assigned apps on startup</p>
                <p class="text-[0.65rem] text-zf-faint">Arrange the selected profile when ZoneFlow starts.</p>
              </div>
              <Switch checked={settings.auto_launch_on_startup} label="Launch assigned apps on startup" onchange={(v) => (settings.auto_launch_on_startup = v)} />
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-close">When closing the editor</Label>
              <Select
                id="set-close"
                value={options.close_action}
                options={[
                  { value: 'ask', label: 'Ask every time' },
                  { value: 'minimize', label: 'Minimize to tray' },
                  { value: 'exit', label: 'Exit ZoneFlow' },
                ]}
                onchange={(v) => (options.close_action = v as CloseAction)}
              />
            </div>
            <p class="rounded-md border border-zf-border bg-zf-bg px-3 py-2 text-[0.7rem] leading-relaxed text-zf-faint">
              Closing the browser editor never stops the background process. Exit is the only action that shuts ZoneFlow down.
            </p>
          </div>
        </section>

        <!-- Overlay -->
        <section class="rounded-lg border border-zf-border bg-zf-surface p-4">
          <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-zf-accent">Overlay</h3>
          <div class="flex flex-col gap-3">
            <div class="flex flex-col gap-1.5">
              <Label for="set-alpha-idle">Inactive alpha ({settings.overlay_inactive_alpha})</Label>
              <input id="set-alpha-idle" type="range" min="0" max="255" value={settings.overlay_inactive_alpha}
                class="h-1.5 w-full cursor-pointer accent-zf-accent"
                oninput={(e) => (settings.overlay_inactive_alpha = num((e.currentTarget as HTMLInputElement).value, settings.overlay_inactive_alpha))} />
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-alpha-active">Active alpha ({settings.overlay_active_alpha})</Label>
              <input id="set-alpha-active" type="range" min="0" max="255" value={settings.overlay_active_alpha}
                class="h-1.5 w-full cursor-pointer accent-zf-accent"
                oninput={(e) => (settings.overlay_active_alpha = num((e.currentTarget as HTMLInputElement).value, settings.overlay_active_alpha))} />
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div class="flex flex-col gap-1.5">
                <Label for="set-debounce">Display debounce (ms)</Label>
                <input id="set-debounce" type="number" min="0" max="5000" value={settings.display_debounce_ms}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.display_debounce_ms = num((e.currentTarget as HTMLInputElement).value, settings.display_debounce_ms))} />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-retries">DWM snap retries</Label>
                <input id="set-retries" type="number" min="0" max="20" value={settings.dwm_snap_retries}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.dwm_snap_retries = num((e.currentTarget as HTMLInputElement).value, settings.dwm_snap_retries))} />
              </div>
            </div>
          </div>
        </section>

        <!-- Application discovery -->
        <section class="rounded-lg border border-zf-border bg-zf-surface p-4">
          <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-zf-accent">Applications</h3>
          <div class="flex flex-col gap-3">
            <div class="flex flex-col gap-1.5">
              <Label for="set-cap">App list cap</Label>
              <input id="set-cap" type="number" min="10" max="2000" value={settings.app_list_cap}
                class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                onchange={(e) => (settings.app_list_cap = num((e.currentTarget as HTMLInputElement).value, settings.app_list_cap))} />
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-ttl">Running apps cache (seconds)</Label>
              <input id="set-ttl" type="number" min="0" max="600" value={settings.running_apps_ttl_secs}
                class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                onchange={(e) => (settings.running_apps_ttl_secs = num((e.currentTarget as HTMLInputElement).value, settings.running_apps_ttl_secs))} />
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div class="flex flex-col gap-1.5">
                <Label for="set-taskbar">Taskbar height</Label>
                <input id="set-taskbar" type="number" min="0" max="200" step="0.5" value={settings.taskbar_h}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.taskbar_h = num((e.currentTarget as HTMLInputElement).value, settings.taskbar_h))} />
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-stage">Stage lower height</Label>
                <input id="set-stage" type="number" min="0" max="200" step="0.5" value={settings.stage_lower_h}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.stage_lower_h = num((e.currentTarget as HTMLInputElement).value, settings.stage_lower_h))} />
              </div>
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-minwin">Minimum window size (px)</Label>
              <input id="set-minwin" type="number" min="0" max="500" value={settings.min_window_px}
                class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                onchange={(e) => (settings.min_window_px = num((e.currentTarget as HTMLInputElement).value, settings.min_window_px))} />
            </div>
          </div>
        </section>
      </div>
    {/if}
  </div>
</div>
