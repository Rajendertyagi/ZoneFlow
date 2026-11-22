/**
 * Pure tests for the per-zone results projection (Finding 3).
 * `apply-results.ts` has no Svelte runes, so bun can import it directly.
 */
import { describe, expect, test } from 'bun:test';
import {
  isApplyResult,
  outcomeTone,
  toOutcomeRows,
} from '../src/lib/state/apply-results';
import { summarizeApplyResult } from '../src/lib/state/apply-result';
import type {
  ApplyResultDto,
  InactiveResult,
  ZoneOutcomeDto,
  ZoneOutcomeStatus,
} from '../src/lib/api/client';

function outcome(
  id: number,
  status: ZoneOutcomeStatus,
  label = '',
  exe = 'app.exe',
  message = 'detail',
): ZoneOutcomeDto {
  return { zone_id: id, label, exe, status, message, launched: false };
}

function result(outcomes: ZoneOutcomeDto[]): ApplyResultDto {
  return {
    profile: 'Coding',
    intent: 'launch_arrange',
    outcomes,
    message: 'summary',
    succeeded: 0,
    failed: 0,
    skipped: 0,
  };
}

describe('toOutcomeRows', () => {
  test('shows name, human outcome and failure reason per zone in order', () => {
    const rows = toOutcomeRows(
      result([
        outcome(1, 'snapped', 'Editor', 'code.exe', 'snapped to 0,0 50x100'),
        outcome(2, 'ambiguous_path', 'Chrome', 'chrome.exe', 'matches two installs'),
      ]),
    );
    expect(rows).toHaveLength(2);
    expect(rows[0]).toMatchObject({
      zoneId: 1,
      name: 'Editor',
      outcome: 'Snapped',
      reason: 'snapped to 0,0 50x100',
      tone: 'success',
    });
    expect(rows[1]).toMatchObject({
      zoneId: 2,
      name: 'Chrome',
      outcome: 'Ambiguous path',
      reason: 'matches two installs',
      tone: 'error',
    });
  });

  test('falls back to exe then zone id when the label is empty', () => {
    const rows = toOutcomeRows(
      result([outcome(3, 'not_running', '', 'app.exe'), outcome(4, 'unassigned', '', '')]),
    );
    expect(rows[0].name).toBe('app.exe');
    expect(rows[1].name).toBe('Zone 4');
  });

  test('empty outcomes produce no rows', () => {
    expect(toOutcomeRows(result([]))).toEqual([]);
  });
});

describe('outcomeTone', () => {
  test('successes, skipped and failures map to distinct tones', () => {
    expect(outcomeTone('snapped')).toBe('success');
    expect(outcomeTone('launched_and_snapped')).toBe('success');
    expect(outcomeTone('unassigned')).toBe('warning');
    expect(outcomeTone('ambiguous_path')).toBe('error');
    expect(outcomeTone('launch_failed')).toBe('error');
    expect(outcomeTone('error')).toBe('error');
  });
});

describe('isApplyResult', () => {
  test('accepts a real apply result', () => {
    expect(isApplyResult(result([]))).toBe(true);
  });

  test('rejects inactive and ok results', () => {
    const inactive: InactiveResult = {
      ok: true,
      applied: false,
      inactive: true,
      reason: 'layout_disabled',
      profile: 'Coding',
    };
    expect(isApplyResult(inactive)).toBe(false);
    expect(isApplyResult({ ok: true })).toBe(false);
    expect(isApplyResult(null)).toBe(false);
  });
});

/**
 * B-1: a per-profile Apply must be reported with the same honest summary/panel
 * as doApply, never a canned success toast. These are the exact pure helpers
 * `guardedProfileOp` now composes (the Svelte store wiring is not importable in
 * a bun test, so the mapping is asserted at helper level).
 */
describe('guarded profile apply reporting (B-1)', () => {
  test('an all-failing ApplyResult is detected and summarized as error', () => {
    const failing = result([
      outcome(1, 'no_eligible_window'),
      outcome(2, 'launch_failed'),
    ]);
    expect(isApplyResult(failing)).toBe(true);
    const summary = summarizeApplyResult(failing);
    expect(summary.tone).toBe('error');
    expect(summary.message).not.toContain('success');
  });

  test('a partially failing ApplyResult is summarized as warning', () => {
    const partial = result([outcome(1, 'snapped'), outcome(2, 'not_running')]);
    expect(isApplyResult(partial)).toBe(true);
    expect(summarizeApplyResult(partial).tone).toBe('warning');
  });

  test('isApplyResult distinguishes an ApplyResult from a plain OkResult', () => {
    expect(isApplyResult({ ok: true })).toBe(false);
    expect(isApplyResult(result([outcome(1, 'snapped')]))).toBe(true);
  });
});
