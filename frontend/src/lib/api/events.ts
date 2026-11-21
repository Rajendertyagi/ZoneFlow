/**
 * Server-Sent Events transport (backend → UI).
 *
 * One EventSource per client, authenticated by the `zf_session` cookie. The
 * backend emits `{ "type": ..., "payload": ... }` frames; `state-changed` has an
 * empty payload and is a signal to re-fetch the full snapshot (state is a full
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

/** Re-fetch the authoritative snapshot. 401 is handled by the client hook. */
export async function refreshState(): Promise<void> {
  try {
    setSnapshot(await api.getState());
  } catch {
    // Transient fetch failure: the next SSE event or reconnect will retry.
  }
}

function handleEvent(event: SseEvent): void {
  noteEvent();
  // Every backend mutation broadcasts `state-changed`; the other types are
  // advisory. A full snapshot refresh is correct for all of them.
  void refreshState();
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
  // missed. Re-fetch the authoritative snapshot on every open/reconnect; this
  // also surfaces a session-expired 401 via the client's fetch hook.
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
  source = new EventSource('/api/events', { withCredentials: true });
  source.onopen = onOpen;
  source.onmessage = onMessage;
  source.onerror = onError;
}

/** Open the event stream. Call once after a successful bootstrap. */
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
