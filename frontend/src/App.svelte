<script lang="ts">
  import { onMount } from 'svelte';
  import { CircleCheck, CircleAlert, Info, TriangleAlert, X } from '@lucide/svelte';
  import { readViewFromHash } from '$lib/api/client';
  import {
    refreshState,
    setSseEventHandler,
    startEvents,
    stopEvents,
    type SseEvent,
  } from '$lib/api/events';
  import { isDirty, store } from '$lib/state/app.svelte';
  import { getCanvasCommands } from '$lib/state/canvas-commands';
  import { resolveUndoShortcut } from '$lib/state/undo-shortcut';
  import {
    dismissToast,
    navigate,
    resolveConfirm,
    resolvePrompt,
    toast,
    ui,
  } from '$lib/state/ui.svelte';
  import TopBar from '$lib/components/TopBar.svelte';
  import ConfirmDialog from '$lib/components/dialogs/ConfirmDialog.svelte';
  import PromptDialog from '$lib/components/dialogs/PromptDialog.svelte';
  import WizardDialog from '$lib/components/dialogs/WizardDialog.svelte';
  import PickerDialog from '$lib/components/dialogs/PickerDialog.svelte';
  import CloseDialog from '$lib/components/dialogs/CloseDialog.svelte';
  import HomeView from '$lib/views/HomeView.svelte';
  import EditorView from '$lib/views/EditorView.svelte';
  import HotkeysView from '$lib/views/HotkeysView.svelte';
  import SettingsView from '$lib/views/SettingsView.svelte';

  let wizardOpen = $state(false);
  let pickerOpen = $state(false);
  let pickerZoneId = $state<number | null>(null);
  let closeOpen = $state(false);

  const pickerZone = $derived(
    pickerZoneId === null
      ? null
      : (store.snapshot?.zones.find((z) => z.id === pickerZoneId) ?? null),
  );

  function onBeforeUnload(event: BeforeUnloadEvent): void {
    // Native browser guard: never lose a dirty draft silently. The in-app Close
    // dialog offers the full Minimize / Exit / Cancel choice (§2a).
    if (isDirty()) {
      event.preventDefault();
      event.returnValue = '';
    }
  }

  function onSseEvent(event: SseEvent): void {
    // Advisory events are real backend output — surface them, never invent them.
    if (event.type === 'apply-result' && event.payload && typeof event.payload === 'object') {
      const p = event.payload as { profile?: string; message?: string };
      toast('info', `Apply result · ${p.profile ?? ''}: ${p.message ?? ''}`.trim());
    } else if (event.type === 'hotkey-action' && typeof event.payload === 'string') {
      toast('info', `Hotkey: ${event.payload}`);
    } else if (event.type === 'log' && event.payload && typeof event.payload === 'object') {
      const p = event.payload as { level?: string; message?: string };
      if (p.level === 'error' || p.level === 'warn') {
        toast(p.level === 'error' ? 'error' : 'warning', p.message ?? 'Backend warning');
      }
    }
  }

  /**
   * Global undo/redo shortcuts. The pure helper gates on view, dialogs, typing
   * target and can_undo/can_redo; the canvas registry supplies the real logic.
   */
  function onGlobalKeydown(event: KeyboardEvent): void {
    const action = resolveUndoShortcut(
      {
        key: event.key,
        ctrlKey: event.ctrlKey,
        metaKey: event.metaKey,
        shiftKey: event.shiftKey,
        altKey: event.altKey,
        target: event.target,
      },
      {
        view: ui.view,
        canUndo: store.snapshot?.can_undo ?? false,
        canRedo: store.snapshot?.can_redo ?? false,
        dialogOpen:
          wizardOpen ||
          pickerOpen ||
          closeOpen ||
          ui.confirmRequest !== null ||
          ui.promptRequest !== null,
      },
    );
    if (action === null) return;
    const commands = getCanvasCommands();
    if (!commands) return;
    event.preventDefault();
    if (action === 'undo') commands.undo();
    else commands.redo();
  }

  onMount(() => {
    setSseEventHandler(onSseEvent);
    window.addEventListener('beforeunload', onBeforeUnload);
    window.addEventListener('keydown', onGlobalKeydown);
    initialize();
    return () => {
      window.removeEventListener('beforeunload', onBeforeUnload);
      window.removeEventListener('keydown', onGlobalKeydown);
      setSseEventHandler(null);
      stopEvents();
    };
  });

  /** Apply a view requested via `#view=…` (tray "Settings…"). */
  function applyInitialView(view: string | null): void {
    if (view === 'home' || view === 'editor' || view === 'hotkeys' || view === 'settings') {
      navigate(view);
    }
  }

  /** Load directly: no login, bootstrap or token exchange phase. */
  function initialize(): void {
    applyInitialView(readViewFromHash());
    void refreshState();
    startEvents();
  }

  function openPicker(zoneId: number | null): void {
    pickerZoneId = zoneId;
    pickerOpen = true;
  }

  function startDraw(): void {
    ui.canvasMode = 'draw';
    navigate('editor');
  }
</script>

<svelte:head>
  <title>ZoneFlow Studio</title>
</svelte:head>

<div class="flex h-screen min-h-[560px] flex-col overflow-hidden bg-zf-bg text-zf-text">
  <TopBar onclose={() => (closeOpen = true)} />

  {#if store.connection === 'lost'}
    <div role="alert" class="mx-5 mt-3 rounded-md border border-zf-warn/40 bg-zf-warn/10 px-4 py-2 text-sm text-zf-warn">
      Connection lost — showing the last known state.
    </div>
  {/if}

    <div class="min-h-0 flex-1">
      {#if ui.view === 'home'}
        <HomeView />
      {:else if ui.view === 'editor'}
        <EditorView onwizard={() => (wizardOpen = true)} onpick={openPicker} />
      {:else if ui.view === 'hotkeys'}
        <HotkeysView />
      {:else}
        <SettingsView />
      {/if}
    </div>

    <!-- Global dialogs -->
    <ConfirmDialog request={ui.confirmRequest} onresolve={resolveConfirm} />
    <PromptDialog request={ui.promptRequest} onresolve={resolvePrompt} />
    <WizardDialog open={wizardOpen} onclose={() => (wizardOpen = false)} ondraw={startDraw} />
    <PickerDialog open={pickerOpen} zone={pickerZone} onclose={() => (pickerOpen = false)} />
    <CloseDialog open={closeOpen} onclose={() => (closeOpen = false)} />

    <!-- Toasts: real feedback from API results and SSE events -->
    <div class="pointer-events-none fixed bottom-5 right-5 z-[60] flex w-[min(26rem,calc(100vw-2rem))] flex-col gap-2">
      {#each ui.toasts as item (item.id)}
        <div
          role="status"
          class="pointer-events-auto flex items-start gap-2 rounded-md border bg-zf-panel px-3 py-2.5 text-xs shadow-xl {item.kind === 'error'
            ? 'border-zf-error/50 text-zf-error'
            : item.kind === 'warning'
              ? 'border-zf-warn/50 text-zf-warn'
              : item.kind === 'success'
                ? 'border-zf-ok/50 text-zf-ok'
                : 'border-zf-accent/50 text-zf-accent'}"
        >
          <span class="mt-0.5 shrink-0">
            {#if item.kind === 'success'}<CircleCheck size={15} />
            {:else if item.kind === 'error'}<CircleAlert size={15} />
            {:else if item.kind === 'warning'}<TriangleAlert size={15} />
            {:else}<Info size={15} />{/if}
          </span>
          <span class="min-w-0 flex-1 whitespace-pre-line leading-relaxed">{item.message}</span>
          <button
            type="button"
            aria-label="Dismiss notification"
            class="shrink-0 rounded p-0.5 opacity-60 hover:opacity-100"
            onclick={() => dismissToast(item.id)}
          >
            <X size={13} />
          </button>
        </div>
      {/each}
    </div>
</div>
