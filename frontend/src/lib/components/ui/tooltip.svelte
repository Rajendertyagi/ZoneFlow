<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';
  import { FOCUSABLE_SELECTOR, isDismissKey } from './a11y-helpers';

  interface Props {
    text: string;
    class?: string;
    children?: Snippet;
  }

  let { text, class: className = '', children }: Props = $props();

  const uid = $props.id();
  const tooltipId = `zf-tooltip-${uid}`;
  let visible = $state(false);
  let wrapperEl: HTMLSpanElement | null = $state(null);

  // Wire aria-describedby to the first focusable descendant (the real trigger).
  // If the children are not focusable, make the wrapper itself the trigger so
  // keyboard users can still reach the description. The description node is
  // always rendered (see `hidden` below), so this reference is never dangling.
  $effect(() => {
    if (!wrapperEl) return;
    const trigger = wrapperEl.querySelector<HTMLElement>(FOCUSABLE_SELECTOR) ?? wrapperEl;
    const isWrapper = trigger === wrapperEl;
    if (isWrapper) trigger.tabIndex = 0;
    trigger.setAttribute('aria-describedby', tooltipId);
    return () => {
      trigger.removeAttribute('aria-describedby');
      if (isWrapper) trigger.removeAttribute('tabindex');
    };
  });

  function onKeydown(event: KeyboardEvent): void {
    if (isDismissKey(event.key) && visible) {
      event.preventDefault();
      visible = false;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<span
  bind:this={wrapperEl}
  role="presentation"
  class={cn(
    'relative inline-flex focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary',
    className,
  )}
  onmouseenter={() => (visible = true)}
  onmouseleave={() => (visible = false)}
  onfocusin={() => (visible = true)}
  onfocusout={(event) => {
    if (!wrapperEl?.contains(event.relatedTarget as Node | null)) visible = false;
  }}
>
  {@render children?.()}
  <!-- Always in the DOM so aria-describedby never points at a missing id.
       `hidden` removes it from rendering and the a11y tree until focus/hover. -->
  <span
    id={tooltipId}
    role="tooltip"
    hidden={!visible}
    class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-1 -translate-x-1/2 whitespace-nowrap rounded border border-input bg-popover px-2 py-1 text-xs text-foreground shadow-lg"
  >
    {text}
  </span>
</span>
