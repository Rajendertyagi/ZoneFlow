/**
 * Pure projection of hotkey registration failures (Finding U-03).
 *
 * The native thread writes failed bindings into a shared slot that the worker
 * copies into `StateSnapshot.hotkey_failures`. This module turns that list into
 * a persistent warning string; it has no Svelte runes so `bun test` can import
 * it directly.
 */

/**
 * Build the persistent warning shown in the Hotkeys view. Returns `null` when
 * there is nothing to report so callers can simply use `{#if warning}`.
 */
export function hotkeyFailureWarning(failures: string[] | undefined): string | null {
  if (!failures || failures.length === 0) return null;
  return `These hotkeys could not be registered: ${failures.join(', ')}.`;
}
