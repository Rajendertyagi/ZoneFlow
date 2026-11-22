/**
 * Pure workflow / dirty-state tests. Run with `bun test` — no new dependencies
 * and no Svelte compiler needed (the module under test has no runes).
 */
import { describe, expect, test } from 'bun:test';
import {
  actionEligibility,
  canRemoveZone,
  duplicateAssignments,
  hasNonDefaultRules,
  isDirtyError,
  isInactiveResult,
  needsReplaceConfirmation,
  previewIsSafe,
  stripAppIdentity,
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
    key_delay: 80, // Zone::default().key_delay on the backend
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

  test('same-exe re-assignment WITH custom rules prompts (rules are reset)', () => {
    const z = zone({ url: 'https://x' });
    expect(needsReplaceConfirmation(z, { label: 'Chrome', exe: 'chrome.exe', path: 'C:\\new\\chrome.exe' })).toBe(true);
  });

  test('same-exe re-assignment with default rules does not prompt', () => {
    const z = zone();
    expect(needsReplaceConfirmation(z, { label: 'Chrome', exe: 'chrome.exe', path: 'C:\\new\\chrome.exe' })).toBe(false);
  });

  test('different app with custom rules prompts', () => {
    const z = zone({ url: 'https://x' });
    expect(needsReplaceConfirmation(z, { label: 'Code', exe: 'Code.exe', path: '' })).toBe(true);
  });

  test('empty zone with custom rules still prompts (assignment resets those rules)', () => {
    const z = zone({ exe: '', url: 'https://x' });
    expect(needsReplaceConfirmation(z, { label: 'Code', exe: 'Code.exe', path: '' })).toBe(true);
  });
});

describe('D4 inactive result', () => {
  test('recognises the backend disabled response', () => {
    expect(
      isInactiveResult({ ok: true, applied: false, inactive: true, reason: 'layout_disabled', profile: 'Website' }),
    ).toBe(true);
  });

  test('does not misclassify a real apply result', () => {
    expect(
      isInactiveResult({
        profile: 'Website',
        intent: 'apply',
        outcomes: [],
        message: 'no zones in profile',
        succeeded: 0,
        failed: 0,
        skipped: 0,
      }),
    ).toBe(false);
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

  test('distinct full paths for the same exe are NOT duplicates', () => {
    const zones = [
      zone({ id: 1, exe: 'chrome.exe', path: 'C:\\Program Files\\Chrome\\chrome.exe' }),
      zone({ id: 2, exe: 'chrome.exe', path: 'D:\\Portable\\Chrome\\chrome.exe' }),
    ];
    expect(duplicateAssignments(zones)).toEqual([]);
  });

  test('same exe + same path IS a duplicate', () => {
    const zones = [
      zone({ id: 1, exe: 'chrome.exe', path: 'C:\\Apps\\chrome.exe' }),
      zone({ id: 2, exe: 'chrome.exe', path: 'C:\\Apps\\chrome.exe' }),
    ];
    expect(duplicateAssignments(zones)).toEqual([{ exe: 'chrome.exe', zoneIds: [1, 2] }]);
  });

  test('an exe-only zone in the group IS a duplicate (path fallback can collide)', () => {
    const zones = [
      zone({ id: 1, exe: 'chrome.exe', path: 'C:\\Apps\\chrome.exe' }),
      zone({ id: 2, exe: 'chrome.exe', path: '' }),
    ];
    expect(duplicateAssignments(zones)).toEqual([{ exe: 'chrome.exe', zoneIds: [1, 2] }]);
  });

  test('paths are normalized (/ → \\, lowercase) before comparison', () => {
    const zones = [
      zone({ id: 1, exe: 'chrome.exe', path: 'C:/Apps/chrome.exe' }),
      zone({ id: 2, exe: 'chrome.exe', path: 'c:\\apps\\CHROME.EXE' }),
    ];
    expect(duplicateAssignments(zones)).toEqual([{ exe: 'chrome.exe', zoneIds: [1, 2] }]);
  });
});

describe('F4 inspector identity fields are display-only', () => {
  test('stripAppIdentity removes exe and path from an inspector patch', () => {
    expect(stripAppIdentity({ exe: 'evil.exe', path: 'C:\\evil.exe', label: 'Editor', url: 'https://x' })).toEqual({
      label: 'Editor',
      url: 'https://x',
    });
  });

  test('stripAppIdentity leaves a patch without identity fields unchanged', () => {
    expect(stripAppIdentity({ launch_mode: 'attach', notes: 'n' })).toEqual({
      launch_mode: 'attach',
      notes: 'n',
    });
  });
});

describe('F3 both delete surfaces may empty a profile', () => {
  test('removal is allowed for 0, 1 and many zones (toolbar + context menu)', () => {
    expect(canRemoveZone(0)).toBe(true);
    expect(canRemoveZone(1)).toBe(true);
    expect(canRemoveZone(5)).toBe(true);
  });
});
