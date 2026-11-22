/**
 * Policy for which SSE events warrant a full snapshot refresh.
 *
 * Only `state-changed` signals that authoritative state was mutated. All other
 * event types (`log`, `status`, `hotkey-action`, `apply-result`, and any future
 * advisory type) are informational: they must NOT trigger `GET /api/state`.
 * Refreshing on advisory events would re-fetch on every frame and, because a
 * fetch used to emit `state-changed`, feed an unbounded request loop.
 *
 * This module is intentionally pure and free of Svelte imports so `bun test`
 * can import it directly (type-only imports would be fine, but none are needed).
 */
export function shouldRefreshOnEvent(type: string): boolean {
  return type === 'state-changed';
}
