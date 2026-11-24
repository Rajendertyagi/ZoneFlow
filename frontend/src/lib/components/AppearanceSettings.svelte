<script lang="ts">
  /**
   * Appearance controls for ModeWatcher-backed mode and preset preferences.
   *
   * Deliberately isolated from the workspace settings draft: this component
   * imports no store/api modules and never calls setSettings/setAppOptions/save.
   * Changing mode or preset only writes ModeWatcher's localStorage keys and
   * cannot dirty the draft or create undo/redo history.
   *
   * Preset identities (documentation note only — no behaviour here): the two
   * built-in JSON presets are `zoneflow-amber` (amber / deep-slate) and
   * `caffeine` (warmer coffee / cream). This component only selects one via
   * ModeWatcher; it never imports, derives or applies theme values itself, so
   * the low-chrome surface/elevation treatment stays entirely in the presets.
   */
  import { setMode, setTheme, theme, userPrefersMode } from 'mode-watcher';
  import { PRESET_MANIFEST, resolvePresetId } from '../../themes/manifest';
  import type { AppearanceMode } from '$lib/theme/preference';
  import Label from '$lib/components/ui/label.svelte';
  import Select from '$lib/components/ui/select.svelte';

  const MODES: readonly { value: AppearanceMode; label: string }[] = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
  ];

  // UI-only mirrors of ModeWatcher's persisted state. They are kept in sync by
  // the effects below and never feed into the workspace draft/save flow.
  let selectedMode = $state<AppearanceMode>(userPrefersMode.current ?? 'system');
  let selectedPreset = $state<string>(resolvePresetId(theme.current));

  $effect(() => {
    selectedMode = userPrefersMode.current ?? 'system';
  });
  $effect(() => {
    selectedPreset = resolvePresetId(theme.current);
  });

  function chooseMode(value: AppearanceMode): void {
    setMode(value);
    selectedMode = value;
  }

  function choosePreset(value: string): void {
    const id = resolvePresetId(value);
    setTheme(id);
    selectedPreset = id;
  }
</script>

<section class="rounded-lg border border-zf-border bg-zf-surface p-4" aria-labelledby="appearance-heading">
  <h3 id="appearance-heading" class="mb-3 text-xs font-semibold uppercase tracking-wider text-zf-accent">
    Appearance
  </h3>
  <div class="flex flex-col gap-3">
    <div class="flex flex-col gap-1.5">
      <span id="appearance-mode-label" class="text-xs font-medium text-zf-muted">Mode</span>
      <div
        class="inline-flex w-fit rounded-md border border-zf-border bg-zf-bg p-0.5"
        role="group"
        aria-labelledby="appearance-mode-label"
      >
        {#each MODES as item (item.value)}
          <button
            type="button"
            aria-pressed={selectedMode === item.value}
            class="rounded px-3 py-1 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zf-accent {selectedMode === item.value
              ? 'bg-zf-accent text-zf-accent-ink'
              : 'text-zf-muted hover:text-zf-text'}"
            onclick={() => chooseMode(item.value)}
          >
            {item.label}
          </button>
        {/each}
      </div>
      <p class="text-[0.65rem] leading-relaxed text-zf-faint">
        System follows your Windows color setting; Light and Dark override it.
      </p>
    </div>

    <div class="flex flex-col gap-1.5">
      <Label for="appearance-preset">Color preset</Label>
      <Select
        id="appearance-preset"
        value={selectedPreset}
        options={PRESET_MANIFEST.map((item) => ({ value: item.id, label: item.label }))}
        onchange={choosePreset}
      />
      <p class="text-[0.65rem] leading-relaxed text-zf-faint">
        Applies instantly to the editor. Native overlay zone colors are controlled separately above.
      </p>
    </div>

    <p class="rounded-md border border-zf-border bg-zf-bg px-3 py-2 text-[0.7rem] leading-relaxed text-zf-faint">
      Appearance is stored in this browser profile only (localStorage). It is not part of your
      workspace draft, does not affect Save/Discard or undo/redo, and does not sync across devices
      or browser profiles.
    </p>
  </div>
</section>
