/// <reference types="node" />
// @ts-ignore - bun:test types are provided by the Bun runtime, not @types/node.
import { beforeEach, describe, expect, mock, test } from 'bun:test';

/**
 * Focused regression tests for the SSE transport's error handling (F1) and
 * idempotent connect (F2). `events.ts` imports a Svelte rune module, so the
 * store-facing imports are mocked here and a minimal EventSource double is
 * installed before the module is loaded.
 */

const calls = { failures: 0, connections: [] as string[] };

mock.module('$lib/state/app.svelte', () => ({
  noteEvent: () => {},
  noteSseFailure: () => ++calls.failures,
  noteSseOpen: () => {},
  setConnection: (state: string) => calls.connections.push(state),
  setSnapshot: () => {},
}));
mock.module('./client', () => ({ api: { getState: async () => ({}) } }));
mock.module('$lib/state/sse-policy', () => ({ shouldRefreshOnEvent: () => false }));

class FakeEventSource {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSED = 2;
  static instances: FakeEventSource[] = [];

  readyState = FakeEventSource.CONNECTING;
  onopen: (() => void) | null = null;
  onmessage: ((ev: MessageEvent<string>) => void) | null = null;
  onerror: (() => void) | null = null;
  closeCount = 0;

  constructor(public url: string) {
    FakeEventSource.instances.push(this);
  }

  close(): void {
    this.closeCount += 1;
    this.readyState = FakeEventSource.CLOSED;
  }
}

(globalThis as any).EventSource = FakeEventSource;
(globalThis as any).window = { setTimeout: () => 1, clearTimeout: () => {} };

const { startEvents, stopEvents } = await import('./events');

function freshSource(): FakeEventSource {
  stopEvents(); // clears any pending reconnect timer from a previous test
  calls.failures = 0;
  calls.connections.length = 0;
  FakeEventSource.instances.length = 0;
  startEvents();
  return FakeEventSource.instances[FakeEventSource.instances.length - 1]!;
}

describe('SSE error classification (F1)', () => {
  test('errors while CONNECTING never count a failure or set lost', () => {
    const src = freshSource();
    src.readyState = FakeEventSource.CONNECTING;
    for (let i = 0; i < 3; i++) src.onerror?.();

    expect(calls.failures).toBe(0);
    expect(calls.connections).toEqual([
      'connecting',
      'reconnecting',
      'reconnecting',
      'reconnecting',
    ]);
  });

  test('errors while OPEN never count a failure either', () => {
    const src = freshSource();
    src.readyState = FakeEventSource.OPEN;
    src.onerror?.();

    expect(calls.failures).toBe(0);
    expect(calls.connections).toEqual(['connecting', 'reconnecting']);
  });

  test('an error while CLOSED counts a failure', () => {
    const src = freshSource();
    src.readyState = FakeEventSource.CLOSED;
    src.onerror?.();

    expect(calls.failures).toBe(1);
  });
});

describe('SSE connect lifecycle (F2)', () => {
  test('starting twice closes the previous EventSource', () => {
    const first = freshSource();
    startEvents();
    const second = FakeEventSource.instances[FakeEventSource.instances.length - 1]!;

    expect(first.closeCount).toBe(1);
    expect(second).not.toBe(first);
    expect(second.url).toBe('/api/events');
  });
});
