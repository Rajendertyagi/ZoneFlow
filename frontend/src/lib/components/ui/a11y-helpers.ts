/**
 * Pure, DOM-free helpers shared by the modernized UI primitives (WP3).
 *
 * Keeping the keyboard/focus math here lets `bun test` cover the behaviour
 * without a DOM test harness or new dependencies. The `.svelte` components own
 * only the DOM wiring (querying elements, calling `.focus()`, event handlers).
 */

/** Selector for natively focusable descendants used by the dialog focus trap. */
export const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(', ');

/** Wrap an integer index into `[0, count)`. Returns -1 when there is nothing to wrap. */
export function wrapIndex(index: number, count: number): number {
  if (count <= 0) return -1;
  return ((index % count) + count) % count;
}

/** First (`home`) or last (`end`) index of a non-empty collection. */
export function edgeIndex(edge: 'home' | 'end', count: number): number {
  if (count <= 0) return -1;
  return edge === 'home' ? 0 : count - 1;
}

/** Roving-tabindex step for tabs: move one item forward/backward with wrapping. */
export function rovingStep(current: number, count: number, forward: boolean): number {
  return wrapIndex(current + (forward ? 1 : -1), count);
}

/**
 * Focus-trap decision for a Tab keypress. Returns the index to focus when the
 * browser would otherwise leave the dialog, or `null` when normal Tab handling
 * is fine. `current` is -1 when focus is outside the container.
 */
export function trapTabIndex(current: number, count: number, backwards: boolean): number | null {
  if (count <= 0) return null;
  if (current < 0 || current >= count) return backwards ? count - 1 : 0;
  if (backwards && current === 0) return count - 1;
  if (!backwards && current === count - 1) return 0;
  return null;
}

export interface Disableable {
  disabled?: boolean;
}

/** Indices of enabled items, in source order. */
export function enabledIndices<T extends Disableable>(items: readonly T[]): number[] {
  const result: number[] = [];
  for (let i = 0; i < items.length; i += 1) {
    if (!items[i].disabled) result.push(i);
  }
  return result;
}

/** Move roving focus to the next enabled menu item, wrapping past the ends. */
export function nextEnabledIndex<T extends Disableable>(
  items: readonly T[],
  current: number,
  forward: boolean,
): number {
  const enabled = enabledIndices(items);
  if (enabled.length === 0) return -1;
  const pos = enabled.indexOf(current);
  if (pos === -1) return enabled[0];
  return enabled[wrapIndex(pos + (forward ? 1 : -1), enabled.length)];
}

/** First/last enabled menu item index (used by Home/End). */
export function edgeEnabledIndex<T extends Disableable>(
  items: readonly T[],
  edge: 'home' | 'end',
): number {
  const enabled = enabledIndices(items);
  if (enabled.length === 0) return -1;
  return edge === 'home' ? enabled[0] : enabled[enabled.length - 1];
}

/** True for the key that dismisses transient overlays (tooltip/menu/dialog). */
export function isDismissKey(key: string): boolean {
  return key === 'Escape';
}
