<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Columns2,
    GitMerge,
    Grid2x2,
    MousePointer2,
    PenTool,
    Plus,
    Redo2,
    Rows2,
    Scissors,
    Trash,
    Undo2,
  } from '@lucide/svelte';
  import Badge from '$lib/components/ui/badge.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import {
    bootstrap,
    canRetryBootstrap,
    getBootstrapMessage,
    retryBootstrap,
    setSessionExpiredHandler,
  } from '$lib/api/client';
  import { refreshState, startEvents, stopEvents } from '$lib/api/events';
  import {
    getApplied,
    getProfiles,
    getSelected,
    isDirty,
    isEnabled,
    markSessionExpired,
    store,
  } from '$lib/state/app.svelte';
  import ZoneCanvas from '$lib/components/Canvas/ZoneCanvas.svelte';

  type Phase = 'loading' | 'ready' | 'error';
  type Mode = 'layout' | 'draw';

  let phase = $state<Phase>('loading');
  let bootstrapMessage = $state('');
  let retryAvailable = $state(false);
  let mode = $state<Mode>('layout');
  let showGap = $state(true);
  let splitRatio = $state(0.5);
  let notice = $state('');
  let noticeTimer: number | null = null;
  let canvas: ZoneCanvas | undefined = $state();

  const sampleApps = [
    { label: 'Chrome', exe: 'chrome.exe', path: '' },
    { label: 'VS Code', exe: 'Code.exe', path: '' },
    { label: 'Terminal', exe: 'WindowsTerminal.exe', path: '' },
  ];

  onMount(() => {
    setSessionExpiredHandler(markSessionExpired);
    void initialize();
    return () => {
      stopEvents();
      if (noticeTimer !== null) window.clearTimeout(noticeTimer);
    };
  });

  async function initialize(): Promise<void> {
    if (!(await bootstrap())) {
      showBootstrapError();
      return;
    }
    phase = 'ready';
    await refreshState();
    startEvents();
  }

  async function retry(): Promise<void> {
    phase = 'loading';
    if (!(await retryBootstrap())) {
      showBootstrapError();
      return;
    }
    phase = 'ready';
    await refreshState();
    startEvents();
  }

  function showBootstrapError(): void {
    phase = 'error';
    bootstrapMessage = getBootstrapMessage();
    retryAvailable = canRetryBootstrap();
  }

  function showNotice(message: string): void {
    notice = message;
    if (noticeTimer !== null) window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => (notice = ''), 3200);
  }

  function onAppDragStart(event: DragEvent, app: (typeof sampleApps)[number]): void {
    const payload = JSON.stringify({ label: app.label, exe: app.exe, path: app.path });
    event.dataTransfer?.setData('application/x-zoneflow-app', payload);
    event.dataTransfer?.setData('text/plain', payload);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
  }

  const connectionLabel = $derived(
    phase === 'error'
      ? 'auth error'
      : store.connection === 'connected'
        ? 'connected'
        : store.connection === 'lost'
          ? 'connection lost'
          : store.connection === 'reconnecting'
            ? 'reconnecting'
            : 'connecting',
  );

  const connectionVariant = $derived(
    phase === 'error' || store.connection === 'lost'
      ? 'error'
      : store.connection === 'connected'
        ? 'success'
        : 'warning',
  );

  const zoneCount = $derived(store.snapshot?.zones.length ?? 0);
  const workArea = $derived(store.snapshot?.status.work_area ?? null);
  const selectedId = $derived(canvas?.getSelectedId() ?? null);
  const canUndo = $derived(store.snapshot?.can_undo ?? false);
  const canRedo = $derived(store.snapshot?.can_redo ?? false);
</script>

<svelte:head>
  <title>ZoneFlow Studio</title>
</svelte:head>

<div class="flex min-h-screen flex-col bg-zf-bg text-zf-text">
  <!-- Top bar -->
  <header class="flex flex-wrap items-center justify-between gap-3 border-b border-zf-border px-5 py-3">
    <div class="flex items-baseline gap-3">
      <h1 class="text-base font-semibold tracking-tight">ZoneFlow <span class="text-zf-accent">Studio</span></h1>
      <span class="hidden text-xs text-zf-faint sm:inline">Layout editor</span>
    </div>
    <div class="flex flex-wrap items-center gap-2 text-xs">
      {#if isDirty()}
        <Badge variant="warning">unsaved</Badge>
      {/if}
      <span class="text-zf-muted">
        <span class="text-zf-faint">profile</span>
        {getApplied() || '—'}
      </span>
      <span class="text-zf-muted">
        <span class="text-zf-faint">selected</span>
        {getSelected() || '—'}
      </span>
      <span class="text-zf-muted">
        <span class="text-zf-faint">engine</span>
        {isEnabled() ? 'enabled' : 'disabled'}
      </span>
      <Badge variant={connectionVariant}>{connectionLabel}</Badge>
    </div>
  </header>

  {#if phase === 'error'}
    <div class="p-6">
      <div role="alert" class="max-w-xl rounded-md border border-zf-error/40 bg-zf-error/10 p-4 text-sm text-zf-error">
        <p class="font-medium">Session unavailable</p>
        <p class="mt-1 text-zf-text/80">{bootstrapMessage}</p>
        {#if retryAvailable}
          <div class="mt-3"><Button size="sm" onclick={retry}>Retry</Button></div>
        {/if}
      </div>
    </div>
  {:else if phase === 'loading'}
    <div class="p-6">
      <div class="rounded-md border border-zf-border bg-zf-panel p-4 text-sm text-zf-muted">Opening session…</div>
    </div>
  {:else}
    {#if store.authError}
      <div role="alert" class="mx-5 mt-3 rounded-md border border-zf-error/40 bg-zf-error/10 px-4 py-2 text-sm text-zf-error">
        {store.authError}
      </div>
    {/if}
    {#if store.connection === 'lost'}
      <div role="alert" class="mx-5 mt-3 rounded-md border border-zf-warn/40 bg-zf-warn/10 px-4 py-2 text-sm text-zf-warn">
        Connection lost — showing the last known state.
      </div>
    {/if}

    <!-- Toolbar -->
    <div class="flex flex-wrap items-center gap-2 border-b border-zf-border bg-zf-surface px-4 py-2">
      <div class="flex items-center rounded-md border border-zf-border bg-zf-bg p-0.5" role="group" aria-label="Canvas mode">
        <button
          type="button"
          aria-pressed={mode === 'layout'}
          onclick={() => (mode = 'layout')}
          class="inline-flex h-7 items-center gap-1.5 rounded px-2.5 text-xs font-medium transition-colors aria-pressed:bg-zf-accent aria-pressed:text-zf-accent-ink text-zf-muted hover:text-zf-text"
        >
          <MousePointer2 size={14} /> Layout
        </button>
        <button
          type="button"
          aria-pressed={mode === 'draw'}
          onclick={() => (mode = 'draw')}
          class="inline-flex h-7 items-center gap-1.5 rounded px-2.5 text-xs font-medium transition-colors aria-pressed:bg-zf-accent aria-pressed:text-zf-accent-ink text-zf-muted hover:text-zf-text"
        >
          <PenTool size={14} /> Draw
        </button>
      </div>

      <span class="mx-1 h-5 w-px bg-zf-border" aria-hidden="true"></span>

      <Button size="sm" variant="outline" title="Add a zone in the first free grid cell" onclick={() => canvas?.addZone()}>
        <Plus size={14} /> Add
      </Button>
      <Button size="sm" variant="ghost" title="Undo (Ctrl+Z in the app)" disabled={!canUndo} onclick={() => canvas?.undo()}>
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

      <span class="ml-auto hidden items-center gap-1.5 text-xs text-zf-faint lg:flex">
        <Scissors size={13} /> drag dividers to resize · right-click for actions
      </span>
    </div>

    <!-- Canvas: the primary center surface -->
    <main class="flex min-h-0 flex-1 flex-col px-4 py-3">
      <div class="relative min-h-[360px] flex-1 overflow-hidden rounded-lg border border-zf-border bg-zf-surface shadow-[0_0_0_1px_rgba(0,0,0,0.2)]">
        <ZoneCanvas bind:this={canvas} {mode} {showGap} {splitRatio} onnotice={showNotice} />
        <div class="pointer-events-none absolute bottom-2 left-3 rounded bg-zf-bg/70 px-2 py-1 font-mono text-[10px] text-zf-faint">
          {mode === 'draw' ? 'DRAW · drag on the grid to paint a zone' : 'LAYOUT · drag body to move · drag divider to resize'}
        </div>
      </div>
    </main>

    <!-- Status / app tray -->
    <footer class="flex flex-wrap items-center gap-x-5 gap-y-2 border-t border-zf-border bg-zf-surface px-5 py-2 text-xs text-zf-muted">
      <span><span class="text-zf-faint">zones</span> <span class="font-mono text-zf-text">{zoneCount}</span></span>
      <span><span class="text-zf-faint">work area</span> <span class="font-mono text-zf-text">{workArea ? `${workArea.w}×${workArea.h}` : '—'}</span></span>
      <span><span class="text-zf-faint">selected</span> <span class="font-mono text-zf-text">{selectedId ?? '—'}</span></span>
      <span class="ml-auto flex items-center gap-2">
        <span class="text-zf-faint">drag an app onto a zone</span>
        {#each sampleApps as app (app.exe)}
          <span
            draggable="true"
            role="button"
            tabindex="0"
            title={`Drag ${app.label} onto a zone`}
            ondragstart={(event) => onAppDragStart(event, app)}
            class="cursor-grab rounded border border-zf-border-strong bg-zf-panel px-2 py-1 text-zf-text active:cursor-grabbing"
          >
            {app.label}
          </span>
        {/each}
      </span>
    </footer>

    {#if notice}
      <div class="pointer-events-none fixed bottom-16 right-5 z-50 rounded-md border border-zf-accent/50 bg-zf-panel px-3 py-2 text-xs text-zf-accent shadow-xl">
        {notice}
      </div>
    {/if}
  {/if}
</div>
