/**
 * Pure workflow / dirty-state tests. Run with `bun test` — no new dependencies
 * and no Svelte compiler needed (the module under test has no runes).
 */
import { describe, expect, test } from 'bun:test';
import {
  actionEligibility,
  duplicateAssignments,
  hasNonDefaultRules,
  isDirtyError,
  isInactiveResult,
  needsReplaceConfirmation,
  previewIsSafe,
  switchDecision,
  switchVerb,
} from '../src/lib/state/workflow';
import type { ZoneDto } from '../src/lib/api/client';

function zone(overrides: Partial<ZoneDto> = {}): ZoneDto {
  return {
    id: 1,
    label: 'Zone',
    exe: 'chrome.exe',
    path: '',
    launch_mode: 'launch',
    title_match: '',
    url: '',
    tab_behavior: 'focus_or_new',
    macro_enabled: false,
    trigger_title: '',
    macro_text: '',
    notes: '',
    key_delay: 0,
    x: 0,
    y: 0,
    w: 50,
    h: 100,
    ...overrides,
  };
}

describe('D1/D2/D4 action eligibility', () => {
  test('clean + enabled allows Apply and Launch & Arrange', () => {
    expect(actionEligibility(false, true)).toEqual({ allowed: true, reason: null });
  });

  test('dirty blocks both actions with the exact D2 explanation', () => {
    const result = actionEligibility(true, true);
    expect(result.allowed).toBe(false);
    expect(result.reason).toBe('Save or revert your changes before applying.');
  });

  test('disabled blocks both actions with the D4 explanation', () => {
    const result = actionEligibility(false, false);
    expect(result.allowed).toBe(false);
    expect(result.reason).toBe('Layout is disabled — enable it to apply.');
  });

  test('dirty takes precedence over disabled in the explanation', () => {
    expect(actionEligibility(true, false).reason).toContain('Save or revert');
  });
});

describe('D2 dirty switch', () => {
  test('clean switch executes without a prompt', () => {
    expect(switchDecision(false, false)).toBe('execute');
  });

  test('dirty switch requires confirmation', () => {
    expect(switchDecision(true, false)).toBe('confirm');
  });

  test('dirty switch with explicit discard executes', () => {
    expect(switchDecision(true, true)).toBe('execute');
  });

  test('dirty_state error is recognised for the retry path', () => {
    expect(isDirtyError({ code: 'dirty_state' })).toBe(true);
    expect(isDirtyError({ code: 'bad_request' })).toBe(false);
    expect(isDirtyError(null)).toBe(false);
  });
});

describe('D3 replacement rules', () => {
  test('default rules need no confirmation', () => {
    expect(hasNonDefaultRules(zone())).toBe(false);
    expect(needsReplaceConfirmation(zone(), { label: 'X', exe: 'code.exe', path: '' })).toBe(false);
  });

  test('each non-default rule is detected', () => {
    expect(hasNonDefaultRules(zone({ launch_mode: 'attach' }))).toBe(true);
    expect(hasNonDefaultRules(zone({ url: 'https://x' }))).toBe(true);
    expect(hasNonDefaultRules(zone({ title_match: 'X' }))).toBe(true);
    expect(hasNonDefaultRules(zone({ tab_behavior: 'reuse_blank' }))).toBe(true);
    expect(hasNonDefaultRules(zone({ macro_enabled: true }))).toBe(true);
    expect(hasNonDefaultRules(zone({ trigger_title: 'Main' }))).toBe(true);
    expect(hasNonDefaultRules(zone({ macro_text: '^t' }))).toBe(true);
    expect(hasNonDefaultRules(zone({ notes: 'x' }))).toBe(true);
    expect(hasNonDefaultRules(zone({ key_delay: 10 }))).toBe(true);
  });

  test('same app re-assignment does not prompt even with custom rules', () => {
    const z = zone({ url: 'https://x' });
    expect(needsReplaceConfirmation(z, { label: 'Chrome', exe: 'chrome.exe', path: '' })).toBe(false);
  });

  test('different app with custom rules prompts', () => {
    const z = zone({ url: 'https://x' });
    expect(needsReplaceConfirmation(z, { label: 'Code', exe: 'Code.exe', path: '' })).toBe(true);
  });

  test('empty zone with custom rules does not prompt (no app identity to replace)', () => {
    const z = zone({ exe: '', url: 'https://x' });
    expect(needsReplaceConfirmation(z, { label: 'Code', exe: 'Code.exe', path: '' })).toBe(false);
  });
});

describe('D4 inactive result', () => {
  test('recognises the backend disabled response', () => {
    expect(
      isInactiveResult({ ok: true, applied: false, inactive: true, reason: 'layout_disabled', profile: 'Website' }),
    ).toBe(true);
  });

  test('does not misclassify a real apply result', () => {
    expect(isInactiveResult({ profile: 'Website', message: 'ok', launched: false })).toBe(false);
    expect(isInactiveResult(undefined)).toBe(false);
  });
});

describe('D5 preview', () => {
  test('preview is only safe when executed is false', () => {
    expect(previewIsSafe(false)).toBe(true);
    expect(previewIsSafe(true)).toBe(false);
  });
});

describe('D6 switch surfaces', () => {
  test('rail loads a draft, home selects only', () => {
    expect(switchVerb('rail', true)).toContain('load draft');
    expect(switchVerb('home', true)).toContain('select only');
  });

  test('tray applies when enabled, selects only when disabled', () => {
    expect(switchVerb('tray', true)).toContain('apply immediately');
    expect(switchVerb('tray', false)).toContain('select only');
  });
});

describe('§3b duplicate assignment warning', () => {
  test('detects the same exe in two zones (case-insensitive)', () => {
    const zones = [zone({ id: 1, exe: 'chrome.exe' }), zone({ id: 2, exe: 'CHROME.EXE' })];
    expect(duplicateAssignments(zones)).toEqual([{ exe: 'chrome.exe', zoneIds: [1, 2] }]);
  });

  test('ignores empty assignments and distinct apps', () => {
    const zones = [zone({ id: 1, exe: '' }), zone({ id: 2, exe: 'a.exe' }), zone({ id: 3, exe: 'b.exe' })];
    expect(duplicateAssignments(zones)).toEqual([]);
  });

  test('reports multiple duplicate groups', () => {
    const zones = [
      zone({ id: 1, exe: 'a.exe' }),
      zone({ id: 2, exe: 'a.exe' }),
      zone({ id: 3, exe: 'b.exe' }),
      zone({ id: 4, exe: 'b.exe' }),
    ];
    expect(duplicateAssignments(zones)).toHaveLength(2);
  });
});
