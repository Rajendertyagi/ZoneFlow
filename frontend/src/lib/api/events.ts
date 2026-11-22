/**
 * Server-Sent Events transport (backend → UI).
 *
 * One EventSource per client, with no credentials of any kind. The backend
 * emits `{ "type": ..., "payload": ... }` frames; `state-changed` has an empty
 * payload and is a signal to re-fetch the full snapshot (state is a full
 * refresh, so replaying deltas would be wrong).
 */

import {
  noteEvent,
  noteSseFailure,
  noteSseOpen,
  setConnection,
  setSnapshot,
} from '$lib/state/app.svelte';
import { api } from './client';
import { shouldRefreshOnEvent } from '$lib/state/sse-policy';

export interface SseEvent {
  type:
    | 'state-changed'
    | 'log'
    | 'status'
    | 'hotkey-action'
    | 'apply-result'
    | (string & {});
  payload: unknown;
}

let source: EventSource | null = null;
let stopped = false;
let reconnectTimer: number | null = null;
let reconnectDelayMs = 1000;
const MAX_RECONNECT_DELAY_MS = 15000;

/** Optional UI handler for advisory events (apply-result, log, hotkey-action). */
type SseEventHandler = (event: SseEvent) => void;
let externalHandler: SseEventHandler | null = null;

export function setSseEventHandler(handler: SseEventHandler | null): void {
  externalHandler = handler;
}

/** Re-fetch the authoritative snapshot. */
export async function refreshState(): Promise<void> {
  try {
    setSnapshot(await api.getState());
  } catch {
    // Transient fetch failure: the next SSE event or reconnect will retry.
  }
}

function handleEvent(event: SseEvent): void {
  noteEvent();
  // Only `state-changed` signals a real mutation. Advisory event types must not
  // trigger a full snapshot fetch — doing so on every frame is what let a
  // read-only GET feed back into another `state-changed` event (idle-CPU loop).
  // The backend now breaks the loop at the source; this is defense in depth.
  if (shouldRefreshOnEvent(event.type)) {
    void refreshState();
  }
  // Let the shell turn documented advisory events into real toasts. The current
  // backend only emits `state-changed`, so this is dormant until those types are
  // produced — it never fabricates feedback.
  externalHandler?.(event);
}

function onMessage(ev: MessageEvent<string>): void {
  try {
    const parsed = JSON.parse(ev.data) as SseEvent;
    if (parsed && typeof parsed.type === 'string') {
      handleEvent(parsed);
    }
  } catch {
    // Ignore malformed frames. Keep-alive comments are not delivered as messages.
  }
}

function onOpen(): void {
  reconnectDelayMs = 1000;
  noteSseOpen();
  // The backend only broadcasts on mutation and sends no replay on (re)connect,
  // so any change that happened while the stream was down would otherwise be
  // missed. Re-fetch the authoritative snapshot on every open/reconnect.
  void refreshState();
}

function onError(): void {
  noteSseFailure();
  // EventSource reconnects automatically while CONNECTING/OPEN. Only when the
  // browser has given up (CLOSED) do we drive our own backoff loop.
  if (source?.readyState === EventSource.CLOSED) {
    scheduleReconnect();
  }
}

function scheduleReconnect(): void {
  if (stopped || reconnectTimer !== null) return;
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = null;
    connect();
  }, reconnectDelayMs);
  reconnectDelayMs = Math.min(reconnectDelayMs * 2, MAX_RECONNECT_DELAY_MS);
}

function connect(): void {
  if (stopped) return;
  source = new EventSource('/api/events');
  source.onopen = onOpen;
  source.onmessage = onMessage;
  source.onerror = onError;
}

/** Open the event stream. Call once on startup. */
export function startEvents(): void {
  stopped = false;
  setConnection('connecting');
  connect();
}

/** Close the stream and cancel any pending reconnect (e.g. on teardown). */
export function stopEvents(): void {
  stopped = true;
  if (reconnectTimer !== null) {
    window.clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  source?.close();
  source = null;
}
