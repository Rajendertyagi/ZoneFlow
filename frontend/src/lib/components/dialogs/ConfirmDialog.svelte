<script lang="ts">
  import Button from '$lib/components/ui/button.svelte';
  import type { ConfirmOptions } from '$lib/state/ui.svelte';

  interface Props {
    request: (ConfirmOptions & { id: number }) | null;
    onresolve: (value: boolean) => void;
  }
  let { request, onresolve }: Props = $props();

  function onKeydown(event: KeyboardEvent): void {
    if (request && event.key === 'Escape') onresolve(false);
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if request}
  <!-- Above the other z-50 dialogs: confirm() can be invoked from inside a
       dialog (e.g. the wizard's replace warning, the picker's replace rules). -->
  <div class="fixed inset-0 z-[70] flex items-center justify-center p-4">
    <button
      type="button"
      aria-label="Cancel dialog"
      class="absolute inset-0 cursor-default bg-scrim"
      onclick={() => onresolve(false)}
    ></button>
    <div
      role="dialog"
      aria-modal="true"
      aria-label={request.title}
      class="relative z-10 w-full max-w-md rounded-lg border border-zf-border-strong bg-zf-panel p-6 shadow-2xl"
    >
      <div class="flex items-start gap-3">
        <span
          class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-sm font-bold {request.destructive
            ? 'bg-zf-error/15 text-zf-error'
            : 'bg-zf-accent/15 text-zf-accent'}"
        >
          {request.destructive ? '!' : '?'}
        </span>
        <div class="min-w-0">
          <h2 class="text-base font-semibold text-zf-text">{request.title}</h2>
          {#if request.description}
            <p class="mt-1.5 text-sm leading-relaxed text-zf-muted">{request.description}</p>
          {/if}
        </div>
      </div>
      <div class="mt-6 flex justify-end gap-2">
        <Button variant="ghost" size="sm" onclick={() => onresolve(false)}>
          {request.cancelLabel ?? 'Cancel'}
        </Button>
        <Button
          variant={request.destructive ? 'destructive' : 'default'}
          size="sm"
          onclick={() => onresolve(true)}
        >
          {request.confirmLabel ?? 'Confirm'}
        </Button>
      </div>
    </div>
  </div>
{/if}
