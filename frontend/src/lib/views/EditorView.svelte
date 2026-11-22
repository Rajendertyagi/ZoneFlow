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
  import { assignAppGuarded, doApply, doDisable, doEnable, doPreview, doRevert, doSave } from '$lib/actions';
  import { store } from '$lib/state/app.svelte';
  import { ui } from '$lib/state/ui.svelte';
  import { actionEligibility } from '$lib/state/workflow';
  import Button from '$lib/components/ui/button.svelte';
  import ZoneCanvas from '$lib/components/Canvas/ZoneCanvas.svelte';
  import AppLibrary from '$lib/components/LeftRail/AppLibrary.svelte';
  import ProfilesPanel from '$lib/components/LeftRail/ProfilesPanel.svelte';
  import ZoneInspector from '$lib/components/Inspector/ZoneInspector.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import ApplyResults from '$lib/components/ApplyResults.svelte';

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
  const dirty = $derived(store.snapshot?.dirty ?? false);
  const enabled = $derived(store.snapshot?.enabled ?? false);
  const canUndo = $derived(store.snapshot?.can_undo ?? false);
  const canRedo = $derived(store.snapshot?.can_redo ?? false);
  const eligibility = $derived(actionEligibility(dirty, enabled));
  const selectedId = $derived(canvas?.getSelectedId() ?? null);
  const selectedZone = $derived(zones.find((z) => z.id === selectedId) ?? null);
  const showInspector = $derived(selectedZone !== null && !inspectorHidden);

  function showNotice(message: string): void {
    notice = message;
    if (noticeTimer !== null) window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => (notice = ''), 3200);
  }

  async function onCanvasAssign(zoneId: number, spec: { label: string; exe: string; path: string }): Promise<void> {
    const zone = zones.find((z) => z.id === zoneId);
    if (!zone) return;
    await assignAppGuarded(zone, spec);
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  <!-- Canvas tools -->
  <div class="flex flex-wrap items-center gap-2 border-b border-zf-border bg-zf-surface px-4 py-2">
    <div class="flex items-center rounded-md border border-zf-border bg-zf-bg p-0.5" role="group" aria-label="Canvas mode">
      <button
        type="button"
        aria-pressed={mode === 'layout'}
        onclick={() => setMode('layout')}
        class="inline-flex h-7 items-center gap-1.5 rounded px-2.5 text-xs font-medium transition-colors aria-pressed:bg-zf-accent aria-pressed:text-zf-accent-ink text-zf-muted hover:text-zf-text"
      >
        <MousePointer2 size={14} /> Layout
      </button>
      <button
        type="button"
        aria-pressed={mode === 'draw'}
        onclick={() => setMode('draw')}
        class="inline-flex h-7 items-center gap-1.5 rounded px-2.5 text-xs font-medium transition-colors aria-pressed:bg-zf-accent aria-pressed:text-zf-accent-ink text-zf-muted hover:text-zf-text"
      >
        <PenTool size={14} /> Draw
      </button>
    </div>

    <span class="mx-1 h-5 w-px bg-zf-border" aria-hidden="true"></span>

    <Button size="sm" variant="outline" title="Add a zone in the first free grid cell" onclick={() => canvas?.addZone()}>
      <Plus size={14} /> Add
    </Button>
    <Button size="sm" variant="ghost" title="Undo" disabled={!canUndo} onclick={() => canvas?.undo()}>
      <Undo2 size={14} /> Undo
    </Button>
    <Button size="sm" variant="ghost" title="Redo" disabled={!canRedo} onclick={() => canvas?.redo()}>
      <Redo2 size={14} /> Redo
    </Button>

    <span class="mx-1 h-5 w-px bg-zf-border" aria-hidden="true"></span>

    <Button size="sm" variant="outline" title="Split selected zone left/right" onclick={() => canvas?.splitSelected('x')}>
      <Columns2 size={14} /> Split ↔
    </Button>
    <Button size="sm" variant="outline" title="Split selected zone top/bottom" onclick={() => canvas?.splitSelected('y')}>
      <Rows2 size={14} /> Split ↕
    </Button>
    <Button size="sm" variant="outline" title="Merge: pick a source zone, then a target zone" onclick={() => canvas?.startMerge()}>
      <GitMerge size={14} /> Merge
    </Button>
    <Button size="sm" variant="ghost" title="Remove selected zone" onclick={() => canvas?.removeSelected()}>
      <Trash size={14} /> Remove
    </Button>

    <span class="mx-1 h-5 w-px bg-zf-border" aria-hidden="true"></span>

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
    <button
      type="button"
      aria-pressed={showGap}
      onclick={() => (showGap = !showGap)}
      title="Toggle the configured gap preview between zones"
      class="inline-flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-xs font-medium transition-colors aria-pressed:border-zf-accent/60 aria-pressed:bg-zf-accent/10 aria-pressed:text-zf-accent border-zf-border text-zf-muted hover:text-zf-text"
    >
      <Grid2x2 size={14} /> Gap
    </button>

    <span class="mx-1 h-5 w-px bg-zf-border" aria-hidden="true"></span>

    <Button size="sm" variant="outline" title="Start from a template" onclick={onwizard}>
      <Sparkles size={14} /> Wizard
    </Button>
    <Button size="sm" variant="outline" title="Assign an app to the selected zone" onclick={() => onpick(selectedId)}>
      <Scissors size={14} /> Assign app
    </Button>
    <Button
      size="sm"
      variant="ghost"
      title="Toggle the contextual inspector"
      onclick={() => (inspectorHidden = !inspectorHidden)}
    >
      <PanelRight size={14} /> Inspector
    </Button>

    <span class="ml-auto hidden items-center gap-1.5 text-xs text-zf-faint xl:flex">
      drag dividers to resize · right-click for actions
    </span>
  </div>

  <!-- Action bar: Save / Apply / Launch / Preview / Revert / Disable are distinct verbs -->
  <div class="flex flex-wrap items-center gap-2 border-b border-zf-border bg-zf-bg px-4 py-2">
    <Button size="sm" variant="default" onclick={() => void doSave()}>
      <Save size={14} /> Save
    </Button>
    <Button size="sm" variant="outline" disabled={!eligibility.allowed} title={eligibility.reason ?? 'Arrange running apps'} onclick={() => void doApply(false)}>
      <Play size={14} /> Apply
    </Button>
    <Button size="sm" variant="outline" disabled={!eligibility.allowed} title={eligibility.reason ?? 'Launch missing apps and arrange'} onclick={() => void doApply(true)}>
      <Rocket size={14} /> Launch &amp; Arrange
    </Button>
    <Button size="sm" variant="ghost" onclick={() => void doPreview()}>
      <Eye size={14} /> Preview
    </Button>
    <Button size="sm" variant="ghost" disabled={!dirty} onclick={() => void doRevert()}>
      <RotateCcw size={14} /> Revert
    </Button>
    <span class="mx-1 h-5 w-px bg-zf-border"></span>
    {#if enabled}
      <Button size="sm" variant="ghost" onclick={() => void doDisable()}>Disable layout</Button>
    {:else}
      <Button size="sm" variant="default" onclick={() => void doEnable()}>Enable layout</Button>
    {/if}
    {#if !eligibility.allowed}
      <span class="ml-1 text-xs text-zf-warn">{eligibility.reason}</span>
    {/if}
    {#if dirty}
      <span class="rounded-full bg-zf-warn/15 px-2 py-0.5 text-[0.7rem] font-medium text-zf-warn ring-1 ring-zf-warn/30">unsaved</span>
    {/if}
  </div>

  <!-- Per-zone results from the last Apply / Launch & Arrange (Finding 3) -->
  <ApplyResults />

  <!-- Three-pane workspace: rails · canvas (primary) · contextual inspector -->
  <div class="flex min-h-0 flex-1">
    <div class="hidden w-60 shrink-0 flex-col border-r border-zf-border bg-zf-surface lg:flex">
      <div class="h-2/5 min-h-0 border-b border-zf-border">
        <ProfilesPanel />
      </div>
      <div class="min-h-0 flex-1">
        <AppLibrary onpick={() => onpick(selectedId)} hasSelection={selectedId !== null} />
      </div>
    </div>

    <main class="relative flex min-h-0 min-w-0 flex-1 flex-col p-3">
      <div class="relative min-h-[320px] flex-1 overflow-hidden rounded-lg border border-zf-border bg-zf-surface shadow-[0_0_0_1px_rgba(0,0,0,0.2)]">
        <ZoneCanvas
          bind:this={canvas}
          {mode}
          {showGap}
          {splitRatio}
          onnotice={showNotice}
          onassign={onCanvasAssign}
          onpick={(id) => onpick(id)}
        />
        <div class="pointer-events-none absolute bottom-2 left-3 rounded bg-zf-bg/70 px-2 py-1 font-mono text-[10px] text-zf-faint">
          {mode === 'draw' ? 'DRAW · drag on the grid to paint a zone' : 'LAYOUT · drag body to move · drag divider to resize'}
        </div>
      </div>
    </main>

    {#if showInspector}
      <div class="hidden w-80 shrink-0 xl:block">
        <ZoneInspector zone={selectedZone} onclose={() => (inspectorHidden = true)} />
      </div>
    {/if}
  </div>

  <StatusBar {selectedId} />

  {#if notice}
    <div class="pointer-events-none fixed bottom-16 right-5 z-40 rounded-md border border-zf-accent/50 bg-zf-panel px-3 py-2 text-xs text-zf-accent shadow-xl">
      {notice}
    </div>
  {/if}
</div>
