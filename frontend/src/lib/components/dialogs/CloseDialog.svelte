<script lang="ts">
  import { Minimize2, Power, X } from '@lucide/svelte';
  import { api } from '$lib/api/client';
  import { store } from '$lib/state/app.svelte';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage } from '$lib/state/workflow';
  import Button from '$lib/components/ui/button.svelte';
  import Switch from '$lib/components/ui/switch.svelte';

  interface Props {
    open: boolean;
    onclose: () => void;
  }
  let { open, onclose }: Props = $props();

  let remember = $state(false);
  let busy = $state(false);

  const dirty = $derived(store.snapshot?.dirty ?? false);

  /**
   * Mirror the native close-dialog behaviour: set close_action to the chosen
   * action when remembered (Ask otherwise) and persist the whole draft to disk.
   * This both makes "Remember this choice" survive a restart and honours §2a's
   * documented exception that Exit auto-saves a dirty draft. Returns false on
   * failure so callers never discard data on a failed write.
   */
  async function persist(action: 'minimize' | 'exit'): Promise<boolean> {
    try {
      await api.setAppOptions({
        auto_start: store.snapshot?.draft.auto_start ?? false,
        close_action: remember ? action : 'ask',
        gap: store.snapshot?.draft.gap ?? 8,
      });
      await api.save();
      return true;
    } catch (error) {
      toast('error', `Could not persist the close choice: ${errorMessage(error)}`);
      return false;
    }
  }

  async function minimize(): Promise<void> {
    if (!(await persist('minimize'))) return;
    onclose();
    // Closing a browser tab does not stop the Rust process (architecture §1).
    window.close();
    toast('info', 'Editor closed. ZoneFlow is still running in the tray.');
  }

  async function exit(): Promise<void> {
    busy = true;
    // §2a: Exit with a dirty draft auto-saves it. `persist` saves the whole
    // draft, so no separate save is needed — and a failed save aborts shutdown
    // rather than silently losing the draft.
    if (!(await persist('exit'))) {
      busy = false;
      return;
    }
    try {
      await api.shutdown();
      toast('info', 'ZoneFlow is shutting down.');
    } catch (error) {
      toast('error', `Shutdown failed: ${errorMessage(error)}`);
    } finally {
      busy = false;
      onclose();
    }
  }
</script>

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <button
      type="button"
      aria-label="Cancel close"
      class="absolute inset-0 cursor-default bg-black/70"
      onclick={onclose}
    ></button>
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Close editor"
      class="relative z-10 w-full max-w-md rounded-lg border border-zf-border-strong bg-zf-panel p-6 shadow-2xl"
    >
      <div class="flex items-center justify-between">
        <h2 class="text-base font-semibold text-zf-text">Close the editor</h2>
        <button
          type="button"
          aria-label="Cancel"
          class="rounded p-1 text-zf-faint hover:bg-zf-raised hover:text-zf-text"
          onclick={onclose}
        >
          <X size={16} />
        </button>
      </div>

      <p class="mt-2 text-sm leading-relaxed text-zf-muted">
        {#if dirty}
          You have unsaved changes. <span class="text-zf-warn">Unsaved changes will be saved on exit.</span>
          Closing the editor itself never stops ZoneFlow — it keeps running in the tray.
        {:else}
          ZoneFlow keeps running in the tray when you close this editor.
        {/if}
      </p>

      <div class="mt-5 flex items-center justify-between rounded-md border border-zf-border bg-zf-surface px-3 py-2">
        <span class="text-xs text-zf-muted">Remember this choice</span>
        <Switch checked={remember} label="Remember this choice" onchange={(v) => (remember = v)} />
      </div>

      <div class="mt-6 flex flex-wrap justify-end gap-2">
        <Button variant="ghost" size="sm" onclick={onclose}>Cancel</Button>
        <Button variant="outline" size="sm" onclick={() => void minimize()}>
          <Minimize2 size={14} /> Minimize to tray
        </Button>
        <Button variant="destructive" size="sm" disabled={busy} onclick={() => void exit()}>
          <Power size={14} /> Exit
        </Button>
      </div>
    </div>
  </div>
{/if}
