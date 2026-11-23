<script module lang="ts">
  // Module-level stack of open dialog elements. The last entry is the topmost
  // modal; only it may re-capture focus that has escaped the dialog. This keeps
  // stacked dialogs from fighting over the same Tab keypress while preserving
  // the focus-trap guarantee for the active dialog.
  const openDialogs: HTMLDivElement[] = [];
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';
  import { FOCUSABLE_SELECTOR, trapTabIndex } from './a11y-helpers';

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

  // Stable, instance-unique ids so the dialog has a real accessible name and
  // description instead of relying on aria-label alone.
  const uid = $props.id();
  const titleId = `zf-dialog-title-${uid}`;
  const descriptionId = `zf-dialog-description-${uid}`;

  let dialogEl: HTMLDivElement | null = $state(null);
  let restoreFocusTo: HTMLElement | null = null;

  function close(): void {
    open = false;
    onclose?.();
  }

  function focusable(): HTMLElement[] {
    if (!dialogEl) return [];
    return Array.from(dialogEl.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
      (el) => el.getClientRects().length > 0,
    );
  }

  function onKeydown(event: KeyboardEvent): void {
    if (!open) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== 'Tab') return;
    const active = document.activeElement as HTMLElement | null;
    const inside = !!dialogEl && !!active && dialogEl.contains(active);
    if (!inside) {
      // Focus escaped the dialog. Only the topmost open dialog may pull it
      // back; a lower stacked dialog stays passive so handlers don't fight.
      if (openDialogs[openDialogs.length - 1] !== dialogEl) return;
    }
    const items = focusable();
    if (items.length === 0) {
      event.preventDefault();
      dialogEl?.focus();
      return;
    }
    const current = inside ? items.indexOf(active as HTMLElement) : -1;
    const target = trapTabIndex(current, items.length, event.shiftKey);
    if (target !== null) {
      event.preventDefault();
      items[target].focus();
    }
  }

  // Restore focus to the trigger when the dialog closes (or unmounts).
  $effect(() => {
    if (!open) return;
    restoreFocusTo = document.activeElement as HTMLElement | null;
    return () => {
      restoreFocusTo?.focus();
      restoreFocusTo = null;
    };
  });

  // Deterministic focus-in: this effect depends on both `open` and the bound
  // `dialogEl`, so it re-runs after the {#if open} block has mounted instead of
  // racing it with a microtask. It also owns this dialog's stack entry.
  $effect(() => {
    if (!open || !dialogEl) return;
    const el = dialogEl;
    openDialogs.push(el);
    const items = focusable();
    (items[0] ?? el).focus();
    return () => {
      const index = openDialogs.lastIndexOf(el);
      if (index !== -1) openDialogs.splice(index, 1);
    };
  });
</script>

<!-- Known limitation (WP3): background `inert` and body scroll-lock are
     intentionally deferred. The scrim + focus trap provide modal behaviour
     without mutating the rest of the document. -->
<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <!-- Scrim: `bg-black/70` is an intentional documented exception — the
         preset schema has no scrim token and a theme-derived scrim would be
         unreadable in light presets. tabindex=-1 keeps it out of the trap. -->
    <button
      type="button"
      tabindex="-1"
      aria-label="Close dialog"
      class="absolute inset-0 cursor-default bg-black/70"
      onclick={close}
    ></button>
    <div
      bind:this={dialogEl}
      role="dialog"
      aria-modal="true"
      aria-labelledby={title ? titleId : undefined}
      aria-describedby={description ? descriptionId : undefined}
      aria-label={title ? undefined : 'Dialog'}
      tabindex="-1"
      class={cn(
        'relative z-10 w-full max-w-lg rounded-lg border border-input bg-popover p-6 shadow-2xl focus:outline-none',
        className,
      )}
    >
      {#if title}
        <h2 id={titleId} class="text-lg font-semibold text-foreground">{title}</h2>
      {/if}
      {#if description}
        <p id={descriptionId} class="mt-1 text-sm text-muted-foreground">{description}</p>
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
