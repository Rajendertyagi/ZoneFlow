<script lang="ts">
  import { RotateCcw, Save, Settings2 } from '@lucide/svelte';
  import { api, type AppOptionsDto, type CloseAction, type SettingsDto } from '$lib/api/client';
  import { refreshState } from '$lib/api/events';
  import { store } from '$lib/state/app.svelte';
  import { isValidHexColor } from '$lib/state/settings-helpers';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage } from '$lib/state/workflow';
  import AppearanceSettings from '$lib/components/AppearanceSettings.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import Switch from '$lib/components/ui/switch.svelte';

  // Non-nullable so event handlers stay type-safe; `loaded` gates rendering
  // until the authoritative snapshot has seeded these values. Fields without a
  // UI control are still present here so a save never drops them from config.
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
    const nextSignature = JSON.stringify(snapshot.settings) +
      JSON.stringify(snapshot.draft.auto_start) +
      String(snapshot.auto_start_registered) +
      snapshot.draft.close_action + snapshot.draft.gap;
    if (nextSignature === signature) return;
    signature = nextSignature;
    settings = { ...snapshot.settings };
    loaded = true;
    options = {
      // Seed the toggle from the real registry state, falling back to the
      // config draft. `draft.auto_start` stays the target that save applies.
      auto_start: snapshot.auto_start_registered ?? snapshot.draft.auto_start,
      close_action: snapshot.draft.close_action,
      gap: snapshot.draft.gap,
    };
    snapText = snapshot.settings.snap_points.join(', ');
  });

  function num(value: string, fallback: number): number {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  /** Keep an invalid color from corrupting the palette. */
  function setTint(index: number, value: string): void {
    if (index < 0 || index >= settings.zone_tints.length) return;
    if (!isValidHexColor(value)) return;
    settings.zone_tints[index] = value;
  }

  async function save(): Promise<void> {
    if (!loaded || busy) return;
    busy = true;
    try {
      const snapPoints = snapText
        .split(',')
        .map((s) => Number(s.trim()))
        .filter((n) => Number.isFinite(n) && n >= 0 && n <= 100);
      // One request applies both DTOs and creates exactly one undo point.
      await api.applySettings({ ...settings, snap_points: snapPoints }, options);
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

    <!-- Appearance is browser-local and intentionally outside the draft/save flow. -->
    <AppearanceSettings />

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
                <p class="text-[0.65rem] leading-relaxed text-zf-faint">Number of vertical columns in the snapping grid.</p>
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-rows">Grid rows</Label>
                <input id="set-rows" type="number" min="1" max="24" value={settings.grid_rows}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.grid_rows = num((e.currentTarget as HTMLInputElement).value, settings.grid_rows))} />
                <p class="text-[0.65rem] leading-relaxed text-zf-faint">Number of horizontal rows in the snapping grid.</p>
              </div>
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-snap">Snap points (% · comma separated)</Label>
              <Input id="set-snap" bind:value={snapText} class="font-mono text-xs" />
              <p class="text-[0.65rem] leading-relaxed text-zf-faint">Percent positions where dragged edges snap, separated by commas.</p>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div class="flex flex-col gap-1.5">
                <Label for="set-min">Min zone %</Label>
                <input id="set-min" type="number" min="1" max="50" step="0.5" value={settings.min_zone_pct}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.min_zone_pct = num((e.currentTarget as HTMLInputElement).value, settings.min_zone_pct))} />
                <p class="text-[0.65rem] leading-relaxed text-zf-faint">Smallest zone allowed, as a percent of the work area.</p>
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-gap">Zone gap %</Label>
                <input id="set-gap" type="number" min="0" max="20" value={options.gap}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (options.gap = num((e.currentTarget as HTMLInputElement).value, options.gap))} />
                <p class="text-[0.65rem] leading-relaxed text-zf-faint">Space left between neighboring zones, as a percent of the work area.</p>
              </div>
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-tol">Boundary tolerance %</Label>
              <input id="set-tol" type="number" min="0" max="10" step="0.1" value={settings.boundary_tolerance_pct}
                class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                onchange={(e) => (settings.boundary_tolerance_pct = num((e.currentTarget as HTMLInputElement).value, settings.boundary_tolerance_pct))} />
              <p class="text-[0.65rem] leading-relaxed text-zf-faint">How close an edge must be to a guide before it snaps.</p>
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-undo">Undo depth</Label>
              <input id="set-undo" type="number" min="1" max="200" value={settings.undo_depth}
                class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                onchange={(e) => (settings.undo_depth = num((e.currentTarget as HTMLInputElement).value, settings.undo_depth))} />
              <p class="text-[0.65rem] leading-relaxed text-zf-faint">Maximum number of edits you can undo.</p>
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
                <p class="text-[0.65rem] text-zf-faint">Start ZoneFlow automatically when you sign in to Windows.</p>
              </div>
              <Switch checked={options.auto_start} label="Start with Windows" onchange={(v) => (options.auto_start = v)} />
            </div>
            <p class="text-[0.65rem] leading-relaxed text-zf-faint">
              Automatic launching of assigned apps at startup is not available in this version.
            </p>
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
              <p class="text-[0.65rem] leading-relaxed text-zf-faint">Choose what happens when you close this browser window.</p>
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
              <p class="text-[0.65rem] leading-relaxed text-zf-faint">Transparency of zone outlines when you are not dragging.</p>
            </div>
            <div class="flex flex-col gap-1.5">
              <Label for="set-alpha-active">Active alpha ({settings.overlay_active_alpha})</Label>
              <input id="set-alpha-active" type="range" min="0" max="255" value={settings.overlay_active_alpha}
                class="h-1.5 w-full cursor-pointer accent-zf-accent"
                oninput={(e) => (settings.overlay_active_alpha = num((e.currentTarget as HTMLInputElement).value, settings.overlay_active_alpha))} />
              <p class="text-[0.65rem] leading-relaxed text-zf-faint">Transparency of zone outlines while you drag a window.</p>
            </div>
            <div class="flex flex-col gap-1.5">
              <Label>Zone colors</Label>
              <p class="text-[0.65rem] leading-relaxed text-zf-faint">Colors used for zones in the editor and wizard.</p>
              <div class="grid grid-cols-6 gap-2">
                {#each settings.zone_tints as tint, i (i)}
                  <label class="flex flex-col items-center gap-1">
                    <input
                      type="color"
                      aria-label="Zone color {i + 1}"
                      value={tint}
                      class="h-8 w-full cursor-pointer rounded border border-zf-border bg-zf-bg p-0.5"
                      onchange={(e) => setTint(i, (e.currentTarget as HTMLInputElement).value)}
                    />
                    <span class="text-[0.6rem] text-zf-faint">Zone {i + 1}</span>
                  </label>
                {/each}
              </div>
            </div>
          </div>
        </section>

        <!-- Advanced engine tuning -->
        <section class="rounded-lg border border-zf-border bg-zf-surface p-4">
          <details class="group">
            <summary class="cursor-pointer text-xs font-semibold uppercase tracking-wider text-zf-accent">
              Advanced
            </summary>
            <p class="mt-2 text-[0.65rem] leading-relaxed text-zf-faint">
              Low-level engine tuning. The defaults work for most setups — change these only if you know what they do.
            </p>
            <div class="mt-3 flex flex-col gap-3">
              <div class="flex flex-col gap-1.5">
                <Label for="set-retries">DWM snap retries</Label>
                <input id="set-retries" type="number" min="0" max="20" value={settings.dwm_snap_retries}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.dwm_snap_retries = num((e.currentTarget as HTMLInputElement).value, settings.dwm_snap_retries))} />
                <p class="text-[0.65rem] leading-relaxed text-zf-faint">How many times Windows is asked to place a window before giving up.</p>
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-minwin">Minimum window size (px)</Label>
                <input id="set-minwin" type="number" min="0" max="500" value={settings.min_window_px}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.min_window_px = num((e.currentTarget as HTMLInputElement).value, settings.min_window_px))} />
                <p class="text-[0.65rem] leading-relaxed text-zf-faint">Windows smaller than this are ignored when arranging.</p>
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-cap">App list cap</Label>
                <input id="set-cap" type="number" min="10" max="2000" value={settings.app_list_cap}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.app_list_cap = num((e.currentTarget as HTMLInputElement).value, settings.app_list_cap))} />
                <p class="text-[0.65rem] leading-relaxed text-zf-faint">Maximum number of installed apps shown in the library.</p>
              </div>
              <div class="flex flex-col gap-1.5">
                <Label for="set-ttl">Running apps cache (seconds)</Label>
                <input id="set-ttl" type="number" min="0" max="600" value={settings.running_apps_ttl_secs}
                  class="h-9 rounded-md border border-zf-border bg-zf-bg px-3 text-sm text-zf-text focus-visible:border-zf-accent focus-visible:outline-none"
                  onchange={(e) => (settings.running_apps_ttl_secs = num((e.currentTarget as HTMLInputElement).value, settings.running_apps_ttl_secs))} />
                <p class="text-[0.65rem] leading-relaxed text-zf-faint">How long the running-app list is reused before it is refreshed.</p>
              </div>
            </div>
          </details>
        </section>
      </div>
    {/if}
  </div>
</div>
