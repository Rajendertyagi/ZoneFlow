/**
 * Client-side mirror of the backend's authoritative state.
 *
 * Svelte 5 module rule: a `$state` variable cannot be exported if it is ever
 * reassigned. We export one non-reassigned object and mutate its properties,
 * plus accessor functions. Components read `store.*` directly for reactivity.
 */

import type {
  HotkeyDto,
  LogLineDto,
  ProfileSummary,
  StateSnapshot,
  ZoneDto,
} from '$lib/api/client';

export type ConnectionState = 'connecting' | 'connected' | 'reconnecting' | 'lost';

interface StoreShape {
  /** Full snapshot from GET /api/state; null until the first fetch/SSE refresh. */
  snapshot: StateSnapshot | null;
  connection: ConnectionState;
  /** Bootstrap or session error banner text; null when no banner is shown. */
  authError: string | null;
  /** True after a 401 — the process restarted and the cookie no longer matches. */
  sessionExpired: boolean;
  lastEventAt: number | null;
  /** Consecutive SSE failures; used to decide when to show "connection lost". */
  sseFailures: number;
}

export const store = $state<StoreShape>({
  snapshot: null,
  connection: 'connecting',
  authError: null,
  sessionExpired: false,
  lastEventAt: null,
  sseFailures: 0,
});

/** Dirty flag derived from the authoritative snapshot (D2). */
const dirtyFlag = $derived(store.snapshot?.dirty ?? false);

/* ------------------------------- accessors -------------------------------- */

export function setSnapshot(next: StateSnapshot): void {
  store.snapshot = next;
  store.sessionExpired = false;
  store.authError = null;
}

export function getSnapshot(): StateSnapshot | null {
  return store.snapshot;
}

export function getProfiles(): ProfileSummary[] {
  return store.snapshot?.profiles ?? [];
}

export function getZones(): ZoneDto[] {
  return store.snapshot?.zones ?? [];
}

export function getHotkeys(): HotkeyDto[] {
  return store.snapshot?.hotkeys ?? [];
}

export function getLogs(): LogLineDto[] {
  return store.snapshot?.logs ?? [];
}

export function getApplied(): string {
  return store.snapshot?.applied ?? '';
}

export function getSelected(): string {
  return store.snapshot?.selected ?? '';
}

export function isEnabled(): boolean {
  return store.snapshot?.enabled ?? false;
}

export function isDirty(): boolean {
  return dirtyFlag;
}

export function canUndo(): boolean {
  return store.snapshot?.can_undo ?? false;
}

export function canRedo(): boolean {
  return store.snapshot?.can_redo ?? false;
}

export function setConnection(state: ConnectionState): void {
  store.connection = state;
}

/** Called on every SSE message: the stream is healthy again. */
export function noteEvent(): void {
  store.lastEventAt = Date.now();
  store.sseFailures = 0;
  store.connection = 'connected';
}

/** Called on SSE error/close. Returns the new consecutive-failure count. */
export function noteSseFailure(): number {
  store.sseFailures += 1;
  store.connection = store.sseFailures >= 3 ? 'lost' : 'reconnecting';
  return store.sseFailures;
}

/** Called by the API client on any 401 after bootstrap. Keeps the draft. */
export function markSessionExpired(): void {
  store.sessionExpired = true;
  store.authError = 'Session expired — reopen the editor from the tray.';
}

export function setAuthError(message: string): void {
  store.authError = message;
}

export function clearAuthError(): void {
  store.authError = null;
}
