/**
 * Pure tests for the apply-result toast summary (Decision 6/8).
 * `apply-result.ts` has no Svelte runes, so bun can import it directly.
 */
import { describe, expect, test } from 'bun:test';
import { summarizeApplyResult } from '../src/lib/state/apply-result';
import type {
  ApplyIntent,
  ApplyResultDto,
  InactiveResult,
  ZoneOutcomeDto,
  ZoneOutcomeStatus,
} from '../src/lib/api/client';

function outcome(id: number, status: ZoneOutcomeStatus): ZoneOutcomeDto {
  return {
    zone_id: id,
    label: `Zone ${id}`,
    exe: 'app.exe',
    status,
    message: `detail for zone ${id}`,
    launched: status === 'launched_and_snapped',
  };
}

function result(
  statuses: ZoneOutcomeStatus[],
  intent: ApplyIntent = 'apply',
  message = 'summary from backend',
): ApplyResultDto {
  const outcomes = statuses.map((status, i) => outcome(i + 1, status));
  return {
    profile: 'Coding',
    intent,
    outcomes,
    message,
    succeeded: outcomes.filter((o) => o.status === 'snapped' || o.status === 'launched_and_snapped')
      .length,
    failed: outcomes.filter(
      (o) => o.status !== 'snapped' && o.status !== 'launched_and_snapped' && o.status !== 'unassigned',
    ).length,
    skipped: outcomes.filter((o) => o.status === 'unassigned').length,
  };
}

const inactive: InactiveResult = {
  ok: true,
  applied: false,
  inactive: true,
  reason: 'layout_disabled',
  profile: 'Coding',
};

describe('summarizeApplyResult', () => {
  test('all snapped → success', () => {
    const summary = summarizeApplyResult(result(['snapped', 'snapped']));
    expect(summary.tone).toBe('success');
    expect(summary.message).toContain('Apply · Coding: summary from backend');
  });

  test('launched_and_snapped counts as success', () => {
    expect(summarizeApplyResult(result(['launched_and_snapped'])).tone).toBe('success');
  });

  test('partial success → warning', () => {
    const summary = summarizeApplyResult(result(['snapped', 'no_eligible_window']));
    expect(summary.tone).toBe('warning');
  });

  test('all failed → error', () => {
    const summary = summarizeApplyResult(result(['unresolved', 'launch_failed']));
    expect(summary.tone).toBe('error');
  });

  test('unassigned-only → info', () => {
    const summary = summarizeApplyResult(result(['unassigned', 'unassigned']));
    expect(summary.tone).toBe('info');
  });

  test('unassigned mixed with a real failure → error', () => {
    expect(summarizeApplyResult(result(['unassigned', 'not_running'])).tone).toBe('error');
  });

  test('inactive → no success toast', () => {
    const summary = summarizeApplyResult(inactive);
    expect(summary.tone).not.toBe('success');
    expect(summary.tone).toBe('info');
    expect(summary.message).toContain('disabled');
  });

  test('empty profile → info, not success', () => {
    const summary = summarizeApplyResult(result([]));
    expect(summary.tone).toBe('info');
  });

  test('intent changes the message but not the outcome tone', () => {
    const apply = summarizeApplyResult(result(['snapped'], 'apply'));
    const launch = summarizeApplyResult(result(['snapped'], 'launch_arrange'));
    expect(apply.tone).toBe('success');
    expect(launch.tone).toBe('success');
    expect(apply.message).toContain('Apply ·');
    expect(launch.message).toContain('Launch & Arrange ·');
  });

  test('uses the backend summary verbatim, never a fabricated success', () => {
    const summary = summarizeApplyResult(
      result(['no_eligible_window'], 'launch_arrange', '1 no eligible window'),
    );
    expect(summary.tone).toBe('error');
    expect(summary.message).toBe('Launch & Arrange · Coding: 1 no eligible window');
  });

  test('a launched zone with no eligible window stays an error, never success', () => {
    const apply = result(['no_eligible_window'], 'launch_arrange');
    apply.outcomes[0].launched = true;
    const summary = summarizeApplyResult(apply);
    expect(summary.tone).toBe('error');
    expect(summary.message).not.toContain('success');
  });
});
