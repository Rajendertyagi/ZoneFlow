/**
 * Pure summary of an apply / launch-arrange result (Decision 6).
 *
 * Kept free of Svelte runes so `bun test` can import it directly. The backend
 * owns the per-zone outcomes; this module only decides which toast tone and
 * message the UI should show. It never turns a failed operation into success.
 */

import type {
  ApplyResultDto,
  InactiveResult,
  ZoneOutcomeDto,
  ZoneOutcomeStatus,
} from '../api/client';
import { isInactiveResult } from './workflow';

export type ApplyTone = 'success' | 'warning' | 'error' | 'info';

export interface ApplySummary {
  tone: ApplyTone;
  message: string;
}

const INACTIVE_MESSAGE = 'Layout is disabled — nothing was applied or launched.';

/** Human-readable label for one backend outcome status (per-zone results panel). */
const OUTCOME_LABELS: Record<ZoneOutcomeStatus, string> = {
  snapped: 'Snapped',
  launched_and_snapped: 'Launched & snapped',
  unassigned: 'Unassigned',
  unresolved: 'Unresolved',
  ambiguous_path: 'Ambiguous path',
  not_running: 'Not running',
  no_eligible_window: 'No eligible window',
  launch_failed: 'Launch failed',
  error: 'Error',
};

/** Pure status → human label. Unknown wire values fall back to a readable form. */
export function formatOutcome(status: ZoneOutcomeStatus): string {
  return OUTCOME_LABELS[status] ?? String(status).replace(/_/g, ' ');
}

function isSnapped(status: ZoneOutcomeStatus): boolean {
  return status === 'snapped' || status === 'launched_and_snapped';
}

function verb(result: ApplyResultDto): string {
  return result.intent === 'launch_arrange' ? 'Launch & Arrange' : 'Apply';
}

function detail(result: ApplyResultDto): string {
  return `${verb(result)} · ${result.profile}: ${result.message}`;
}

/**
 * Map a backend result to a toast tone + message.
 *
 * - inactive → info (never a success toast)
 * - all zones snapped → success
 * - some snapped, some not → warning
 * - no successes, at least one real failure → error
 * - only unassigned zones → info
 * - no zones → info
 */
export function summarizeApplyResult(
  result: ApplyResultDto | InactiveResult | unknown,
): ApplySummary {
  if (isInactiveResult(result)) {
    return { tone: 'info', message: INACTIVE_MESSAGE };
  }

  const apply = result as ApplyResultDto;
  const outcomes: ZoneOutcomeDto[] = Array.isArray(apply?.outcomes) ? apply.outcomes : [];

  if (outcomes.length === 0) {
    return { tone: 'info', message: detail(apply) };
  }

  const snapped = outcomes.filter((o) => isSnapped(o.status)).length;
  const unassigned = outcomes.filter((o) => o.status === 'unassigned').length;
  const failures = outcomes.length - snapped - unassigned;

  if (snapped === outcomes.length) {
    return { tone: 'success', message: detail(apply) };
  }
  if (snapped > 0) {
    return { tone: 'warning', message: detail(apply) };
  }
  if (failures > 0) {
    return { tone: 'error', message: detail(apply) };
  }
  // Only unassigned zones remain.
  return { tone: 'info', message: detail(apply) };
}
