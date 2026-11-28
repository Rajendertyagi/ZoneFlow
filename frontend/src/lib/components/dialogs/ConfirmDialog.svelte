<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import type { ConfirmOptions } from '$lib/state/ui.svelte';

  const focusRing = 'focus-visible:outline-2 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-primary focus-visible:ring-0 motion-reduce:transition-none';

  interface Props {
    request: (ConfirmOptions & { id: number }) | null;
    onresolve: (value: boolean) => void;
  }
  let { request, onresolve }: Props = $props();
</script>

<Dialog.Root
  open={!!request}
  onOpenChange={(next) => {
    if (!next) onresolve(false);
  }}
>
  <Dialog.Content
    showCloseButton={false}
    class="z-[70] flex max-w-md flex-col gap-0 rounded-lg border border-zf-border-strong bg-zf-panel p-6 shadow-2xl ring-0 sm:max-w-md"
  >
    <Dialog.Header class="flex flex-row items-start gap-3">
      <span
        class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-sm font-bold {request?.destructive
          ? 'bg-zf-error/15 text-zf-error'
          : 'bg-zf-accent/15 text-zf-accent'}"
      >
        {request?.destructive ? '!' : '?'}
      </span>
      <div class="min-w-0">
        <Dialog.Title class="text-base font-semibold text-zf-text">{request?.title ?? ''}</Dialog.Title>
        {#if request?.description}
          <Dialog.Description class="mt-1.5 text-sm leading-relaxed text-zf-muted">
            {request.description}
          </Dialog.Description>
        {/if}
      </div>
    </Dialog.Header>
    <Dialog.Footer
      class="mx-0 mb-0 mt-6 flex flex-row justify-end gap-2 rounded-none border-t-0 bg-transparent p-0"
    >
      <Button variant="ghost" size="sm" class={`h-7 text-xs ${focusRing}`} onclick={() => onresolve(false)}>
        {request?.cancelLabel ?? 'Cancel'}
      </Button>
      <Button
        variant={request?.destructive ? 'destructive' : 'default'}
        size="sm"
        class={`h-7 text-xs ${focusRing}`}
        onclick={() => onresolve(true)}
      >
        {request?.confirmLabel ?? 'Confirm'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
