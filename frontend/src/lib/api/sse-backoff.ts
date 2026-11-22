/**
 * Pure reconnect-backoff calculation for the SSE transport. Kept free of
 * EventSource/Svelte imports so `bun test` can import it directly.
 *
 * Mirrors the logic in events.ts: start at 1s, double on each failed reconnect
 * attempt, cap at 15s; a successful open resets the delay to 1s.
 */
export const INITIAL_RECONNECT_DELAY_MS = 1000;
export const MAX_RECONNECT_DELAY_MS = 15000;

/** Next delay after a failed reconnect attempt (doubled, capped at 15s). */
export function nextReconnectDelay(currentMs: number): number {
  return Math.min(currentMs * 2, MAX_RECONNECT_DELAY_MS);
}
