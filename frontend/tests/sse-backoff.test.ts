/**
 * Regression tests for the SSE reconnect backoff sequence. The calculation is
 * extracted into a pure module (src/lib/api/sse-backoff.ts) so it can be tested
 * without an EventSource or a DOM. events.ts consumes these exact constants
 * and helper, so a change here would break the transport's real behaviour.
 */
import { describe, expect, test } from 'bun:test';
import {
  INITIAL_RECONNECT_DELAY_MS,
  MAX_RECONNECT_DELAY_MS,
  nextReconnectDelay,
} from '../src/lib/api/sse-backoff';

describe('SSE reconnect backoff', () => {
  test('starts at 1s and doubles up to the 15s cap', () => {
    const sequence: number[] = [INITIAL_RECONNECT_DELAY_MS];
    let delay = INITIAL_RECONNECT_DELAY_MS;
    for (let i = 0; i < 8; i++) {
      delay = nextReconnectDelay(delay);
      sequence.push(delay);
    }
    expect(sequence).toEqual([
      1000, 2000, 4000, 8000, 15000, 15000, 15000, 15000, 15000,
    ]);
  });

  test('never exceeds the cap', () => {
    expect(nextReconnectDelay(MAX_RECONNECT_DELAY_MS)).toBe(MAX_RECONNECT_DELAY_MS);
    expect(nextReconnectDelay(999_999)).toBe(MAX_RECONNECT_DELAY_MS);
  });

  test('reset and cap constants match the documented policy', () => {
    expect(INITIAL_RECONNECT_DELAY_MS).toBe(1000);
    expect(MAX_RECONNECT_DELAY_MS).toBe(15000);
  });
});
