/**
 * Pure product-workflow rules (D1–D6). No Svelte runes, no DOM, no network —
 * this module is directly unit-testable with `bun test` and is the single place
 * where the frozen decisions are encoded on the client side.
 *
 * The backend remains authoritative (see `src/api/commands.rs`); these helpers
 * only decide what the UI is allowed to offer and what copy to show.
 */

import type { AssignDto, InactiveResult, ZoneDto } from '../api/client';

/* ------------------------------- D1 / D2 / D4 ------------------------------ */

export interface Eligibility {
  allowed: boolean;
  /** Human-readable explanation shown next to a disabled action. */
  reason: string | null;
}

/**
 * Apply and Launch & Arrange share the same gate (D1): both are edit verbs that
 * must never run against a stale saved profile while the draft is dirty (D2),
 * and both are suspended while the layout is disabled (D4).
 */
export function actionEligibility(dirty: boolean, enabled: boolean): Eligibility {
  if (dirty) {
    return {
      allowed: false,
      reason: 'Save or revert your changes before applying.',
    };
  }
  if (!enabled) {
    return {
      allowed: false,
      reason: 'Layout is disabled — enable it to apply.',
    };
  }
  return { allowed: true, reason: null };
}

/** D2: a profile switch with a dirty draft needs an explicit Discard/Cancel. */
export type SwitchDecision = 'execute' | 'confirm';

export function switchDecision(dirty: boolean, userConfirmedDiscard: boolean): SwitchDecision {
  return !dirty || userConfirmedDiscard ? 'execute' : 'confirm';
}

/* ---------------------------------- D3 ------------------------------------ */

/**
 * True when a zone carries any application-specific rule that `assignZone`
 * would reset. Used to decide whether D3's confirmation is required.
 */
export function hasNonDefaultRules(zone: ZoneDto): boolean {
  return (
    zone.launch_mode !== 'launch' ||
    zone.url.trim() !== '' ||
    zone.title_match.trim() !== '' ||
    zone.tab_behavior !== 'focus_or_new' ||
    zone.macro_enabled ||
    zone.trigger_title.trim() !== '' ||
    zone.macro_text.trim() !== '' ||
    zone.notes.trim() !== '' ||
    // `Zone::default().key_delay` is 80 on the backend — treat that as the
    // default so a freshly-created zone never triggers a false D3 prompt.
    zone.key_delay !== 80
  );
}

/** D3: replacing a *different* app with custom rules must be confirmed. */
export function needsReplaceConfirmation(zone: ZoneDto, next: AssignDto): boolean {
  const replacingDifferentApp = zone.exe.trim() !== '' && zone.exe !== next.exe;
  return replacingDifferentApp && hasNonDefaultRules(zone);
}

/* ---------------------------------- D4 ------------------------------------ */

/** The backend returns this shape (HTTP 200) instead of applying when disabled. */
export function isInactiveResult(value: unknown): value is InactiveResult {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return v['inactive'] === true && v['applied'] === false;
}

/* ---------------------------------- D5 ------------------------------------ */

/** Preview must never claim execution, whatever the backend returns. */
export function previewIsSafe(executed: boolean): boolean {
  return executed === false;
}

/* ---------------------------------- D6 ------------------------------------ */

export type SwitchSurface = 'rail' | 'home' | 'tray';

/**
 * D6 describes which verb each surface uses. Kept as data so the UI can show a
 * truthful hint and tests can pin the contract down.
 */
export function switchVerb(surface: SwitchSurface, enabled: boolean): string {
  if (surface === 'rail') return 'load draft for editing';
  if (surface === 'home') return 'select only (no apply)';
  return enabled ? 'apply immediately' : 'select only (layout disabled)';
}

/* ------------------------- §3b duplicate assignment ----------------------- */

export interface DuplicateAssignment {
  exe: string;
  zoneIds: number[];
}

/**
 * The backend places a single HWND per exe/path, so two zones with the same app
 * in one profile cannot both be reliably positioned. Detect and warn; never
 * silently claim multi-window support.
 */
export function duplicateAssignments(zones: ZoneDto[]): DuplicateAssignment[] {
  const byExe = new Map<string, number[]>();
  for (const zone of zones) {
    const exe = zone.exe.trim().toLowerCase();
    if (!exe) continue;
    const ids = byExe.get(exe) ?? [];
    ids.push(zone.id);
    byExe.set(exe, ids);
  }
  return [...byExe.entries()]
    .filter(([, ids]) => ids.length > 1)
    .map(([exe, ids]) => ({ exe, zoneIds: ids }));
}

/* ------------------------------- misc helpers ----------------------------- */

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** True when the API rejected an operation because the draft is dirty (D2). */
export function isDirtyError(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    (error as { code?: unknown }).code === 'dirty_state'
  );
}
