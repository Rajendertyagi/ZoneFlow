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

/**
 * D3: any assignment that will reset non-default rules must be confirmed —
 * including re-picking the *same* app. `ZoneAssign` resets all rules
 * unconditionally, so re-assigning the same exe (e.g. to add a path) would
 * otherwise silently destroy launch mode / URL / tab / macro / notes. The
 * geometry, id and position are preserved and the library item is never
 * deleted; only the rules are reset, and only after confirmation.
 */
export function needsReplaceConfirmation(zone: ZoneDto, _next: AssignDto): boolean {
  return hasNonDefaultRules(zone);
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
 * Path-aware port of the backend's `duplicate_app_assignments` (engine_core.rs).
 * Zones are grouped by case-insensitive exe name; within a group, two zones
 * with distinct non-empty full paths are NOT duplicates (exact path matching
 * resolves them to different windows). A group is ambiguous only when at least
 * one zone has no path (it can fall back to the first exe-name match and
 * collide) or when multiple zones share the same path. Paths are normalized
 * (`/` → `\`, lowercased) exactly like the backend before comparison.
 */
export function duplicateAssignments(zones: ZoneDto[]): DuplicateAssignment[] {
  const byExe = new Map<string, ZoneDto[]>();
  for (const zone of zones) {
    const exe = zone.exe.trim().toLowerCase();
    if (!exe) continue;
    const group = byExe.get(exe) ?? [];
    group.push(zone);
    byExe.set(exe, group);
  }
  const out: DuplicateAssignment[] = [];
  for (const [exe, group] of byExe) {
    if (group.length < 2) continue;
    const paths = new Set<string>();
    let hasExeOnly = false;
    for (const zone of group) {
      const path = zone.path.trim();
      if (path === '') {
        hasExeOnly = true;
      } else {
        paths.add(path.replace(/\//g, '\\').toLowerCase());
      }
    }
    // Distinct full paths with no exe-only zone resolve to different windows —
    // not a duplicate. Otherwise the whole exe group is ambiguous.
    if (!hasExeOnly && paths.size > 1) continue;
    out.push({ exe, zoneIds: group.map((z) => z.id) });
  }
  return out;
}

/* --------------------------- inspector field policy ----------------------- */

/**
 * F4: app identity (`exe`/`path`) is display-only in the inspector. Changing it
 * must go through the guarded picker (`assignAppGuarded`) so D3's confirmation
 * and rule reset always run — a direct `ZoneUpdate` would silently keep the old
 * app's URL/title/macro rules. Strip identity fields from any inspector patch
 * as defense in depth, even if a future edit accidentally adds one.
 */
export function stripAppIdentity(patch: Partial<ZoneDto>): Partial<ZoneDto> {
  const safe: Partial<ZoneDto> = { ...patch };
  delete safe.exe;
  delete safe.path;
  return safe;
}

/* ----------------------------- delete surfaces ---------------------------- */

/**
 * F3: both delete surfaces (toolbar Remove and canvas context-menu Remove) may
 * empty a profile — the backend allows deleting the last zone. No count-based
 * guard belongs on either surface; this shared policy keeps them consistent.
 */
export function canRemoveZone(zoneCount: number): boolean {
  // A profile may always be emptied, regardless of how many zones it has.
  return zoneCount >= 0;
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
