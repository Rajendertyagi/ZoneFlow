/**
 * Regression test for the idle-CPU SSE feedback loop.
 *
 * Only `state-changed` may trigger a full snapshot refresh. Advisory event
 * types must never do so, otherwise every frame re-fetches `GET /api/state`
 * and (before the backend fix) re-emits `state-changed` without bound.
 *
 * `sse-policy.ts` is pure and Svelte-free, so bun can import it directly.
 */
import { describe, expect, test } from 'bun:test';
import { shouldRefreshOnEvent } from '../src/lib/state/sse-policy';

describe('shouldRefreshOnEvent', () => {
  test('state-changed warrants a full snapshot refresh', () => {
    expect(shouldRefreshOnEvent('state-changed')).toBe(true);
  });

  test('advisory event types never trigger a refresh', () => {
    for (const type of ['log', 'status', 'hotkey-action', 'apply-result']) {
      expect(shouldRefreshOnEvent(type)).toBe(false);
    }
  });

  test('unknown event types never trigger a refresh', () => {
    expect(shouldRefreshOnEvent('some_future_event')).toBe(false);
    expect(shouldRefreshOnEvent('')).toBe(false);
  });
});
