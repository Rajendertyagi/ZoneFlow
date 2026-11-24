<script lang="ts">
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import type { PromptOptions } from '$lib/state/ui.svelte';

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

  function onKeydown(event: KeyboardEvent): void {
    if (!request) return;
    if (event.key === 'Escape') onresolve(null);
    if (event.key === 'Enter') submit();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if request}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <button
      type="button"
      aria-label="Cancel dialog"
      class="absolute inset-0 cursor-default bg-scrim"
      onclick={() => onresolve(null)}
    ></button>
    <div
      role="dialog"
      aria-modal="true"
      aria-label={request.title}
      class="relative z-10 w-full max-w-md rounded-lg border border-zf-border-strong bg-zf-panel p-6 shadow-2xl"
    >
      <h2 class="text-base font-semibold text-zf-text">{request.title}</h2>
      {#if request.description}
        <p class="mt-1.5 text-sm text-zf-muted">{request.description}</p>
      {/if}
      <div class="mt-4 flex flex-col gap-1.5">
        <Label>{request.label ?? 'Name'}</Label>
        <Input
          bind:value
          placeholder={request.placeholder ?? ''}
          id="zf-prompt-input"
          oninput={() => {}}
        />
      </div>
      <div class="mt-6 flex justify-end gap-2">
        <Button variant="ghost" size="sm" onclick={() => onresolve(null)}>Cancel</Button>
        <Button size="sm" disabled={!value.trim()} onclick={submit}>
          {request.confirmLabel ?? 'Save'}
        </Button>
      </div>
    </div>
  </div>
{/if}
