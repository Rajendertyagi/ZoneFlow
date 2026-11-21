<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  interface Props {
    open?: boolean;
    title?: string;
    description?: string;
    class?: string;
    children?: Snippet;
    footer?: Snippet;
    onclose?: () => void;
  }

  let {
    open = $bindable(false),
    title,
    description,
    class: className = '',
    children,
    footer,
    onclose,
  }: Props = $props();

  function close(): void {
    open = false;
    onclose?.();
  }

  function onKeydown(event: KeyboardEvent): void {
    if (open && event.key === 'Escape') close();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <button
      type="button"
      aria-label="Close dialog"
      class="absolute inset-0 cursor-default bg-black/70"
      onclick={close}
    ></button>
    <div
      role="dialog"
      aria-modal="true"
      aria-label={title}
      class={cn(
        'relative z-10 w-full max-w-lg rounded-lg border border-zf-border-strong bg-zf-panel p-6 shadow-2xl',
        className,
      )}
    >
      {#if title}
        <h2 class="text-lg font-semibold text-zf-text">{title}</h2>
      {/if}
      {#if description}
        <p class="mt-1 text-sm text-zf-muted">{description}</p>
      {/if}
      <div class="mt-4">
        {@render children?.()}
      </div>
      {#if footer}
        <div class="mt-6 flex justify-end gap-2">
          {@render footer?.()}
        </div>
      {/if}
    </div>
  </div>
{/if}
