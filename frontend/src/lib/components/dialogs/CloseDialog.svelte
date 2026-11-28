<script lang="ts">
  import { Minimize2, Power, X } from '@lucide/svelte';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { api } from '$lib/api/client';
  import { store } from '$lib/state/app.svelte';
  import { persistCloseChoice } from '$lib/state/close-action';
  import { toast } from '$lib/state/ui.svelte';
  import { errorMessage } from '$lib/state/workflow';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';

  // Preserve the repo focus-ring / reduced-motion convention over the nova defaults.
  const focusRing = 'focus-visible:outline-2 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-primary focus-visible:ring-0 motion-reduce:transition-none';

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
    return persistCloseChoice(
      {
        auto_start: store.snapshot?.draft.auto_start ?? false,
        gap: store.snapshot?.draft.gap ?? 8,
      },
      action,
      remember,
      {
        onError: (error) =>
          toast('error', `Could not persist the close choice: ${errorMessage(error)}`),
      },
    );
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

<Dialog.Root
  open={open}
  onOpenChange={(next) => {
    if (!next) onclose();
  }}
>
  <Dialog.Content
    showCloseButton={false}
    aria-label="Close editor"
    class="flex max-w-md flex-col gap-0 rounded-lg border border-zf-border-strong bg-zf-panel p-6 shadow-2xl ring-0 sm:max-w-md"
  >
    <Dialog.Header class="flex flex-row items-center justify-between gap-0">
      <Dialog.Title class="text-base font-semibold text-zf-text">Close the editor</Dialog.Title>
      <Dialog.Close>
        {#snippet child({ props })}
          <button
            {...props}
            type="button"
            aria-label="Cancel"
            class="rounded p-1 text-zf-faint hover:bg-zf-raised hover:text-zf-text {focusRing}"
          >
            <X size={16} />
          </button>
        {/snippet}
      </Dialog.Close>
    </Dialog.Header>

    <Dialog.Description class="mt-2 text-sm leading-relaxed text-zf-muted">
      {#if dirty}
        You have unsaved changes. <span class="text-zf-warn">Unsaved changes will be saved on exit.</span>
        Closing the editor itself never stops ZoneFlow — it keeps running in the tray.
      {:else}
        ZoneFlow keeps running in the tray when you close this editor.
      {/if}
    </Dialog.Description>

    <div class="mt-5 flex items-center justify-between rounded-md border border-zf-border bg-zf-surface px-3 py-2">
      <span class="text-xs text-zf-muted">Remember this choice</span>
      <Switch bind:checked={remember} aria-label="Remember this choice" class={focusRing} />
    </div>

    <Dialog.Footer
      class="mx-0 mb-0 mt-6 flex flex-wrap justify-end gap-2 rounded-none border-t-0 bg-transparent p-0"
    >
      <Button variant="ghost" size="sm" class={`h-7 text-xs ${focusRing}`} onclick={onclose}>Cancel</Button>
      <Button variant="outline" size="sm" class={`h-7 text-xs ${focusRing}`} onclick={() => void minimize()}>
        <Minimize2 size={14} /> Minimize to tray
      </Button>
      <Button variant="destructive" size="sm" disabled={busy} class={`h-7 text-xs ${focusRing}`} onclick={() => void exit()}>
        <Power size={14} /> Exit
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
