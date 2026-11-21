<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  interface Props {
    direction?: 'horizontal' | 'vertical';
    /** Initial size of the first pane, as a percentage. */
    initial?: number;
    class?: string;
    start?: Snippet;
    end?: Snippet;
  }

  let {
    direction = 'horizontal',
    initial = 25,
    class: className = '',
    start,
    end,
  }: Props = $props();

  // Intentionally seed from the prop once; subsequent drags own this value.
  // svelte-ignore state_referenced_locally
  let split = $state(initial);
  let dragging = $state(false);
  let container: HTMLDivElement | null = $state(null);

  function onPointerDown(event: PointerEvent): void {
    dragging = true;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: PointerEvent): void {
    if (!dragging || !container) return;
    const rect = container.getBoundingClientRect();
    const pct =
      direction === 'horizontal'
        ? ((event.clientX - rect.left) / rect.width) * 100
        : ((event.clientY - rect.top) / rect.height) * 100;
    split = Math.min(80, Math.max(20, pct));
  }

  function onPointerUp(event: PointerEvent): void {
    dragging = false;
    (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
  }
</script>

<div
  bind:this={container}
  class={cn(
    'flex min-h-0 min-w-0',
    direction === 'horizontal' ? 'flex-row' : 'flex-col',
    className,
  )}
>
  <div class="min-h-0 min-w-0 overflow-auto" style:flex-basis="{split}%">
    {@render start?.()}
  </div>
  <div
    role="separator"
    aria-orientation={direction === 'horizontal' ? 'vertical' : 'horizontal'}
    class={cn(
      'shrink-0 bg-zf-border transition-colors hover:bg-zf-accent',
      direction === 'horizontal' ? 'w-1 cursor-col-resize' : 'h-1 cursor-row-resize',
    )}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
  ></div>
  <div class="min-h-0 min-w-0 flex-1 overflow-auto">
    {@render end?.()}
  </div>
</div>
