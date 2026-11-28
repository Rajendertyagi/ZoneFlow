<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Label } from '$lib/components/ui/label/index.js';
  import type { PromptOptions } from '$lib/state/ui.svelte';

  const focusRing = 'focus-visible:outline-2 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-primary focus-visible:ring-0 motion-reduce:transition-none';

  interface Props {
    request: (PromptOptions & { id: number }) | null;
    onresolve: (value: string | null) => void;
  }
  let { request, onresolve }: Props = $props();

  let value = $state('');
  let lastId = -1;

  // Seed the input whenever a new request arrives (keyed by its unique id).
  $effect(() => {
    if (request && request.id !== lastId) {
      lastId = request.id;
      value = request.initialValue ?? '';
      queueMicrotask(() => document.getElementById('zf-prompt-input')?.focus());
    }
  });

  function submit(): void {
    const trimmed = value.trim();
    if (!trimmed) return;
    onresolve(trimmed);
  }

  // Escape dismissal is native to Bits UI (mapped to onOpenChange -> onresolve(null)).
  // Enter is a prompt-specific shortcut and is kept here.
  function onKeydown(event: KeyboardEvent): void {
    if (!request || event.key !== 'Enter') return;
    const target = event.target as HTMLElement | null;
    if (target?.closest('button, a[href], [role="button"], [role="menuitem"]')) return;
    submit();
  }
</script>

<!-- Modal-only: the handler early-returns when no request is open, and Enter is scoped away from interactive controls. -->
<svelte:window onkeydown={onKeydown} />

<Dialog.Root
  open={!!request}
  onOpenChange={(next) => {
    if (!next) onresolve(null);
  }}
>
  <Dialog.Content
    showCloseButton={false}
    aria-label={request?.title ?? 'Prompt'}
    class="flex max-w-md flex-col gap-0 rounded-lg border border-zf-border-strong bg-zf-panel p-6 shadow-2xl ring-0 sm:max-w-md"
  >
    <Dialog.Header class="gap-1.5">
      <Dialog.Title class="text-base font-semibold text-zf-text">{request?.title ?? ''}</Dialog.Title>
      {#if request?.description}
        <Dialog.Description class="text-sm text-zf-muted">{request.description}</Dialog.Description>
      {/if}
    </Dialog.Header>
    <div class="mt-4 flex flex-col gap-1.5">
      <Label for="zf-prompt-input" class="text-xs text-muted-foreground">{request?.label ?? 'Name'}</Label>
      <Input
        bind:value
        placeholder={request?.placeholder ?? ''}
        id="zf-prompt-input"
        class="border border-border focus-visible:border-primary"
        oninput={() => {}}
      />
    </div>
    <Dialog.Footer
      class="mx-0 mb-0 mt-6 flex flex-row justify-end gap-2 rounded-none border-t-0 bg-transparent p-0"
    >
      <Button variant="ghost" size="sm" class={`h-7 text-xs ${focusRing}`} onclick={() => onresolve(null)}>Cancel</Button>
      <Button size="sm" disabled={!value.trim()} class={`h-9 px-4 ${focusRing}`} onclick={submit}>
        {request?.confirmLabel ?? 'Save'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
