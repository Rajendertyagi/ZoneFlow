/**
 * Tests for pure, user-visible helpers in src/lib/state/ that previously had
 * no coverage. All are Svelte-free and govern labels, messages or warnings the
 * user actually sees, so regressions here are real UX bugs.
 */
import { describe, expect, test } from 'bun:test';
import { hotkeyLabel } from '../src/lib/state/hotkey-labels';
import { compactProfileLoadOptions } from '../src/lib/state/profile-switch';
import { countAssignedZonesToRemove } from '../src/lib/state/wizard';
import { appsOnTabEnter, sourceAfterBrowse } from '../src/lib/state/app-source';
import type { ZoneDto } from '../src/lib/api/client';

function zone(id: number, exe: string): ZoneDto {
  return {
    id,
    exe,
    label: '',
    path: '',
    launcher_path: '',
    launcher_show_console: true,
    launch_mode: 'launch',
    title_match: '',
    url: '',
    tab_behavior: 'focus_or_new',
    macro_enabled: false,
    trigger_title: '',
    macro_text: '',
    notes: '',
    key_delay: 80,
    x: 0,
    y: 0,
    w: 100,
    h: 100,
  };
}

describe('hotkeyLabel', () => {
  test('maps documented action names to friendly labels', () => {
    expect(hotkeyLabel('clear_layout')).toBe('Clear layout');
    expect(hotkeyLabel('move_left')).toBe('Move window left');
    expect(hotkeyLabel('move_right')).toBe('Move window right');
    expect(hotkeyLabel('move_up')).toBe('Move window up');
    expect(hotkeyLabel('move_down')).toBe('Move window down');
    expect(hotkeyLabel('maximize')).toBe('Maximize window');
    expect(hotkeyLabel('restore')).toBe('Restore window');
    expect(hotkeyLabel('reload')).toBe('Reload configuration');
  });

  test('labels profile_N as a zone launch, not a profile apply', () => {
    expect(hotkeyLabel('profile_1')).toBe('Launch/focus zone 1 (current profile)');
    expect(hotkeyLabel('profile_9')).toBe('Launch/focus zone 9 (current profile)');
  });

  test('passes through unknown names and non-single-digit profiles', () => {
    expect(hotkeyLabel('some_custom_action')).toBe('some_custom_action');
    // The regex is single-digit only; profile_10 is not a valid zone binding.
    expect(hotkeyLabel('profile_10')).toBe('profile_10');
  });
});

describe('compactProfileLoadOptions', () => {
  test('uses the same load action and message shape as the rail', () => {
    expect(compactProfileLoadOptions('Coding')).toEqual({
      profile: 'Coding',
      action: 'switch to Coding',
      successMessage: 'Loaded Coding into the editor draft.',
    });
  });
});

describe('countAssignedZonesToRemove', () => {
  test('counts only deleted zones with a non-empty exe', () => {
    const zones = [
      zone(1, 'chrome.exe'), // kept
      zone(2, 'code.exe'), // removed, assigned
      zone(3, ''), // removed, unassigned
      zone(4, '   '), // removed, whitespace-only → unassigned
      zone(5, 'wt.exe'), // removed, assigned
    ];
    expect(countAssignedZonesToRemove(zones, 1)).toBe(2);
  });

  test('returns zero when every deleted zone is unassigned', () => {
    const zones = [zone(1, 'chrome.exe'), zone(2, ''), zone(3, '')];
    expect(countAssignedZonesToRemove(zones, 1)).toBe(0);
  });

  test('returns zero for an empty list', () => {
    expect(countAssignedZonesToRemove([], 1)).toBe(0);
  });
});

describe('app-source helpers', () => {
  test('sourceAfterBrowse keeps the current source unchanged', () => {
    expect(sourceAfterBrowse('installed')).toBe('installed');
    expect(sourceAfterBrowse('running')).toBe('running');
    expect(sourceAfterBrowse('browse')).toBe('browse');
  });

  test('appsOnTabEnter shows an empty list only for the browse tab', () => {
    expect(appsOnTabEnter('browse')).toEqual([]);
    expect(appsOnTabEnter('installed')).toBeNull();
    expect(appsOnTabEnter('running')).toBeNull();
  });
});
