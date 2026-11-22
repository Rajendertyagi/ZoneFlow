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
  lastEventAt: number | null;
  /** Consecutive SSE failures; used to decide when to show "connection lost". */
  sseFailures: number;
}

export const store = $state<StoreShape>({
  snapshot: null,
  connection: 'connecting',
  lastEventAt: null,
  sseFailures: 0,
});

/** Dirty flag derived from the authoritative snapshot (D2). */
const dirtyFlag = $derived(store.snapshot?.dirty ?? false);

/* ------------------------------- accessors -------------------------------- */

export function setSnapshot(next: StateSnapshot): void {
  store.snapshot = next;
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

/**
 * Optimistically patch one zone's geometry in the local mirror after a commit.
 * The authoritative SSE refresh follows immediately; this only removes the
 * one-frame flicker where the zone would snap back to its old position.
 */
export function patchZoneLocal(id: number, patch: Partial<ZoneDto>): void {
  const zone = store.snapshot?.zones.find((z) => z.id === id);
  if (zone) Object.assign(zone, patch);
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

/**
 * Called when the SSE stream (re)opens. Reset the consecutive-failure count so
 * "connection lost" reflects repeated failures, not a lifetime total; a single
 * later blip must not immediately re-trigger the banner after a healthy reopen.
 */
export function noteSseOpen(): void {
  store.sseFailures = 0;
  store.connection = 'connected';
}

/** Called on SSE error/close. Returns the new consecutive-failure count. */
export function noteSseFailure(): number {
  store.sseFailures += 1;
  store.connection = store.sseFailures >= 3 ? 'lost' : 'reconnecting';
  return store.sseFailures;
}


