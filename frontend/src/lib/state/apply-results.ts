/**
 * Pure per-zone apply-result projection (Finding 3).
 *
 * The backend already returns one outcome per zone. This module turns those
 * outcomes into display rows (name / outcome / failure reason) without any
 * Svelte runes, so `bun test` can import it directly. The rune-backed store in
 * `apply-results.svelte.ts` holds the rows; this module holds the logic.
 */

import type { ApplyResultDto, ZoneOutcomeStatus } from '../api/client';
import { formatOutcome } from './apply-result';

export type OutcomeTone = 'success' | 'warning' | 'error';

export interface OutcomeRow {
  zoneId: number;
  /** Application name: label first, exe fallback, then a stable zone label. */
  name: string;
  /** Human-readable outcome status. */
  outcome: string;
  /** Backend failure reason (empty for a clean snap). */
  reason: string;
  status: ZoneOutcomeStatus;
  tone: OutcomeTone;
}

/** Badge tone for one outcome. Success/warning/error only — no silent success. */
export function outcomeTone(status: ZoneOutcomeStatus): OutcomeTone {
  if (status === 'snapped' || status === 'launched_and_snapped') return 'success';
  if (status === 'unassigned') return 'warning';
  return 'error';
}

/** Project a backend apply result into display rows, preserving zone order. */
export function toOutcomeRows(result: ApplyResultDto): OutcomeRow[] {
  return result.outcomes.map((o) => ({
    zoneId: o.zone_id,
    name: o.label.trim() || o.exe.trim() || `Zone ${o.zone_id}`,
    outcome: formatOutcome(o.status),
    reason: o.message,
    status: o.status,
    tone: outcomeTone(o.status),
  }));
}

/** Narrow an API response to a real apply result (vs. inactive / ok results). */
export function isApplyResult(value: unknown): value is ApplyResultDto {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return Array.isArray(v['outcomes']) && typeof v['profile'] === 'string';
}
