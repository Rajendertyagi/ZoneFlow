<script lang="ts">
  import {
    Columns2,
    Eye,
    GitMerge,
    Grid2x2,
    PanelRight,
    MousePointer2,
    PenTool,
    Play,
    Plus,
    Redo2,
    Rocket,
    RotateCcw,
    Rows2,
    Save,
    Scissors,
    Sparkles,
    Trash,
    Undo2,
  } from '@lucide/svelte';
  import { api, type AssignDto } from '$lib/api/client';
  import {
    assignAppGuarded,
    doApply,
    doDisable,
    doEnable,
    doPreview,
    doRevert,
    doSave,
    guardedProfileOp,
  } from '$lib/actions';
  import { getProfiles, store } from '$lib/state/app.svelte';
  import { compactProfileLoadOptions } from '$lib/state/profile-switch';
  import { ui } from '$lib/state/ui.svelte';
  import { actionEligibility, canRemoveZone } from '$lib/state/workflow';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as ButtonGroup from '$lib/components/ui/button-group/index.js';
  import * as ToggleGroup from '$lib/components/ui/toggle-group/index.js';
  import ZoneCanvas from '$lib/components/Canvas/ZoneCanvas.svelte';
  import AppLibrary from '$lib/components/LeftRail/AppLibrary.svelte';
  import ProfilesPanel from '$lib/components/LeftRail/ProfilesPanel.svelte';
  import ZoneInspector from '$lib/components/Inspector/ZoneInspector.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import ApplyResults from '$lib/components/ApplyResults.svelte';

  // Preserve the repo focus-ring / reduced-motion convention over nova defaults.
  const focusRing = 'focus-visible:outline-2 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-primary focus-visible:ring-0 motion-reduce:transition-none';
  const btnSm = `h-7 text-xs ${focusRing}`;
  const btnDefault = `h-9 px-4 ${focusRing}`;

  interface Props {
    onwizard: () => void;
    onpick: (zoneId: number | null) => void;
  }
  let { onwizard, onpick }: Props = $props();

  const mode = $derived(ui.canvasMode);
  function setMode(next: 'layout' | 'draw'): void {
    ui.canvasMode = next;
  }
  let showGap = $state(true);
  let splitRatio = $state(0.5);
  let inspectorHidden = $state(false);
  let notice = $state('');
  let noticeTimer: number | null = null;
  let canvas: ZoneCanvas | undefined = $state();

  const zones = $derived(store.snapshot?.zones ?? []);
  const profiles = $derived(getProfiles());
  const selectedProfile = $derived(store.snapshot?.selected ?? '');
  const dirty = $derived(store.snapshot?.dirty ?? false);
  const enabled = $derived(store.snapshot?.enabled ?? false);
  const canUndo = $derived(store.snapshot?.can_undo ?? false);
  const canRedo = $derived(store.snapshot?.can_redo ?? false);
  const eligibility = $derived(actionEligibility(dirty, enabled));
  const selectedId = $derived(canvas?.getSelectedId() ?? null);
  const selectedZone = $derived(zones.find((z) => z.id === selectedId) ?? null);

  function showNotice(message: string): void {
    notice = message;
    if (noticeTimer !== null) window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => (notice = ''), 3200);
  }

  async function onCanvasAssign(zoneId: number, spec: AssignDto): Promise<void> {
    const zone = zones.find((z) => z.id === zoneId);
    if (!zone) return;
    await assignAppGuarded(zone, spec);
  }

  /** U-07: same load action as the rail's ProfilesPanel, for narrow widths. */
  async function loadCompactProfile(name: string): Promise<void> {
    await guardedProfileOp(
      (discard) => api.loadProfile(name, discard),
      compactProfileLoadOptions(name),
    );
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  <!-- Three-pane workspace: rails · canvas (primary) · contextual inspector -->
  <div class="flex min-h-0 flex-1">
    <div class="hidden w-60 shrink-0 flex-col shadow-zf-rail bg-zf-surface lg:flex">
      <div class="h-2/5 min-h-0 border-b border-zf-border">
        <ProfilesPanel />
      </div>
      <div class="min-h-0 flex-1">
        <AppLibrary onpick={() => onpick(selectedId)} hasSelection={selectedId !== null} />
      </div>
    </div>

    <!-- Middle column: tool rows above the canvas -->
    <div class="flex min-h-0 min-w-0 flex-1 flex-col">
      <!-- Canvas tools -->
      <div class="flex flex-wrap items-center gap-2 shadow-zf-topbar bg-zf-surface px-4 py-2">
        <ToggleGroup.Root
          type="single"
          value={mode}
          onValueChange={(v) => v && setMode(v as 'layout' | 'draw')}
          aria-label="Canvas mode"
          class="rounded-md bg-zf-bg p-0.5 shadow-zf-1"
        >
          <ToggleGroup.Item
            value="layout"
            size="sm"
            class={`h-7 gap-1.5 rounded px-2.5 text-xs font-medium text-zf-muted data-[state=on]:bg-zf-accent data-[state=on]:text-zf-accent-ink hover:text-zf-text ${focusRing}`}
          >
            <MousePointer2 size={14} /> Layout
          </ToggleGroup.Item>
          <ToggleGroup.Item
            value="draw"
            size="sm"
            class={`h-7 gap-1.5 rounded px-2.5 text-xs font-medium text-zf-muted data-[state=on]:bg-zf-accent data-[state=on]:text-zf-accent-ink hover:text-zf-text ${focusRing}`}
          >
            <PenTool size={14} /> Draw
          </ToggleGroup.Item>
        </ToggleGroup.Root>

        <!-- U-07: compact profile selector for widths below `lg`, where the rail
             (and its ProfilesPanel) is hidden. Calls the same load action. -->
        <label class="flex items-center gap-1.5 text-xs text-zf-muted lg:hidden" title="Load a profile into the editor draft">
          Profile
          <select
            aria-label="Profile"
            class="h-7 max-w-40 rounded border border-zf-border bg-zf-bg px-2 text-xs text-zf-text focus:border-zf-accent focus:outline-none"
            value={selectedProfile}
            onchange={(e) => void loadCompactProfile((e.currentTarget as HTMLSelectElement).value)}
          >
            {#each profiles as profile (profile.name)}
              <option value={profile.name}>{profile.name}</option>
            {/each}
          </select>
        </label>

        <span class="mx-2 h-5 w-px bg-zf-border" aria-hidden="true"></span>

        <Button size="sm" variant="outline" title="Add a zone in the first free grid cell" class={btnSm} onclick={() => canvas?.addZone()}>
          <Plus size={14} /> Add
        </Button>
        <ButtonGroup.Root orientation="horizontal" aria-label="Undo and redo">
          <Button size="sm" variant="ghost" title="Undo" disabled={!canUndo} class={btnSm} onclick={() => canvas?.undo()}>
            <Undo2 size={14} /> Undo
          </Button>
          <Button size="sm" variant="ghost" title="Redo" disabled={!canRedo} class={btnSm} onclick={() => canvas?.redo()}>
            <Redo2 size={14} /> Redo
          </Button>
        </ButtonGroup.Root>

        <span class="mx-2 h-5 w-px bg-zf-border" aria-hidden="true"></span>

        <Button size="sm" variant="outline" title="Split selected zone left/right" class={btnSm} onclick={() => canvas?.splitSelected('x')}>
          <Columns2 size={14} /> Split ↔
        </Button>
        <Button size="sm" variant="outline" title="Split selected zone top/bottom" class={btnSm} onclick={() => canvas?.splitSelected('y')}>
          <Rows2 size={14} /> Split ↕
        </Button>
        <Button size="sm" variant="outline" title="Merge: pick a source zone, then a target zone" class={btnSm} onclick={() => canvas?.startMerge()}>
          <GitMerge size={14} /> Merge
        </Button>
        <Button
          size="sm"
          variant="ghost"
          title="Remove selected zone"
          disabled={!canRemoveZone(zones.length)}
          class={btnSm}
          onclick={() => canvas?.removeSelected()}
        >
          <Trash size={14} /> Remove
        </Button>

        <span class="mx-2 h-5 w-px bg-zf-border" aria-hidden="true"></span>

        <label class="flex items-center gap-1.5 text-xs text-zf-muted" title="Ratio kept by the first half when splitting">
          ratio
          <input
            type="number"
            min="0.05"
            max="0.95"
            step="0.05"
            bind:value={splitRatio}
            class="h-7 w-16 rounded border border-zf-border bg-zf-bg px-2 text-right font-mono text-xs text-zf-text focus:border-zf-accent focus:outline-none"
          />
        </label>
        <ToggleGroup.Root
          type="single"
          value={showGap ? 'gap' : ''}
          onValueChange={(v) => (showGap = !!v)}
          aria-label="Gap preview"
        >
          <ToggleGroup.Item
            value="gap"
            size="sm"
            title="Toggle the configured gap preview between zones"
            class={`h-7 gap-1.5 rounded-md bg-zf-surface px-2.5 text-xs font-medium text-zf-muted shadow-zf-1 data-[state=on]:bg-zf-accent/15 data-[state=on]:text-zf-accent hover:bg-zf-raised hover:text-zf-text ${focusRing}`}
          >
            <Grid2x2 size={14} /> Gap
          </ToggleGroup.Item>
        </ToggleGroup.Root>

        <span class="mx-2 h-5 w-px bg-zf-border" aria-hidden="true"></span>

        <Button size="sm" variant="outline" title="Start from a template" class={btnSm} onclick={onwizard}>
          <Sparkles size={14} /> Wizard
        </Button>
        <Button size="sm" variant="outline" title="Assign an app to the selected zone" class={btnSm} onclick={() => onpick(selectedId)}>
          <Scissors size={14} /> Assign app
        </Button>
        <Button
          size="sm"
          variant="ghost"
          title="Toggle the contextual inspector"
          class={btnSm}
          onclick={() => (inspectorHidden = !inspectorHidden)}
        >
          <PanelRight size={14} /> Inspector
        </Button>

        <span class="ml-auto hidden items-center gap-1.5 text-xs text-zf-faint xl:flex">
          drag dividers to resize · right-click for actions
        </span>
      </div>

      <!-- Action bar: Save / Apply / Launch / Preview / Revert / Disable are distinct verbs -->
      <div class="flex flex-wrap items-center gap-2 shadow-zf-topbar bg-zf-bg px-4 py-2">
        <Button size="sm" variant="default" class={btnSm} onclick={() => void doSave()}>
          <Save size={14} /> Save
        </Button>
        <Button size="sm" variant="outline" disabled={!eligibility.allowed} title={eligibility.reason ?? 'Arrange running apps'} class={btnSm} onclick={() => void doApply(false)}>
          <Play size={14} /> Apply
        </Button>
        <Button size="sm" variant="outline" disabled={!eligibility.allowed} title={eligibility.reason ?? 'Launch missing apps and arrange'} class={btnSm} onclick={() => void doApply(true)}>
          <Rocket size={14} /> Launch &amp; Arrange
        </Button>
        <Button size="sm" variant="ghost" class={btnSm} onclick={() => void doPreview()}>
          <Eye size={14} /> Preview
        </Button>
        <Button size="sm" variant="ghost" disabled={!dirty} class={btnSm} onclick={() => void doRevert()}>
          <RotateCcw size={14} /> Revert
        </Button>
        <span class="mx-2 h-5 w-px bg-zf-border"></span>
        {#if enabled}
          <Button size="sm" variant="ghost" class={btnSm} onclick={() => void doDisable()}>Disable layout</Button>
        {:else}
          <Button size="sm" variant="default" class={btnSm} onclick={() => void doEnable()}>Enable layout</Button>
        {/if}
      </div>

      <!-- P3: warning + unsaved status live in one slim banner, not inline with buttons -->
      {#if !eligibility.allowed || dirty}
        <div class="flex items-center gap-2 rounded-md bg-zf-warn/10 px-4 py-2 text-xs text-zf-warn">
          {#if !eligibility.allowed}
            <span>{eligibility.reason}</span>
          {/if}
          {#if dirty}
            <span class="rounded-full bg-zf-warn/15 px-2 py-0.5 font-semibold text-zf-warn ring-1 ring-zf-warn/30">unsaved</span>
          {/if}
        </div>
      {/if}

      <!-- Per-zone results from the last Apply / Launch & Arrange (Finding 3) -->
      <ApplyResults />

      <main class="relative flex min-h-0 min-w-0 flex-1 flex-col p-3">
        <!-- P5: cap the frame to the work-area aspect ratio (9/16 = 0.5625) so
             the `meet`-scaled SVG fills it instead of letterboxing into a void.
             `my-auto` centres the compact frame in the main column; min-h keeps
             a usable floor. Sidebar widths are subtracted per breakpoint:
             rail 240px (lg+), inspector 320px (xl+), p-3 padding 24px. -->
        <div class="relative my-auto min-h-[320px] max-h-[calc((100vw_-_24px)*0.5625)] flex-1 overflow-hidden rounded-lg bg-zf-surface shadow-zf-1 lg:max-h-[calc((100vw_-_264px)*0.5625)] xl:max-h-[calc((100vw_-_584px)*0.5625)]">
          <ZoneCanvas
            bind:this={canvas}
            {mode}
            {showGap}
            {splitRatio}
            onnotice={showNotice}
            onassign={onCanvasAssign}
            onpick={(id) => onpick(id)}
          />
          <div class="pointer-events-none absolute bottom-2 left-3 rounded bg-zf-bg/70 px-2 py-1 font-mono text-xs text-zf-muted">
            {mode === 'draw' ? 'DRAW · drag on the grid to paint a zone' : 'LAYOUT · drag body to move · drag divider to resize'}
          </div>
        </div>
      </main>
    </div>

    {#if !inspectorHidden}
      <!-- Wrapper-owned separation: ZoneInspector's aside intentionally has no
           directional elevation because it is only rendered inside this wrapper. -->
      <div class="hidden w-80 shrink-0 shadow-zf-inspector xl:block">
        <ZoneInspector zone={selectedZone} onclose={() => (inspectorHidden = true)} />
      </div>
    {/if}
  </div>

  <StatusBar {selectedId} />

  {#if notice}
    <div class="pointer-events-none fixed bottom-16 right-5 z-40 rounded-md bg-zf-panel px-3 py-2 text-xs text-zf-accent shadow-zf-2">
      {notice}
    </div>
  {/if}
</div>
