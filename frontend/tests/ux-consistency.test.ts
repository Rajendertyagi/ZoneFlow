/**
 * Pure UX/interaction-consistency helpers (WP5). No Svelte compiler, DOM or
 * network needed — every module under test is rune-free. Run with `bun test`.
 */
import { describe, expect, test } from 'bun:test';
import { cn } from '../src/lib/utils';
import { hotkeyLabel } from '../src/lib/state/hotkey-labels';
import { compactProfileLoadOptions } from '../src/lib/state/profile-switch';
import { countAssignedZonesToRemove } from '../src/lib/state/wizard';
import { appsOnTabEnter, sourceAfterBrowse } from '../src/lib/state/app-source';
import type { ZoneDto } from '../src/lib/api/client';

function zone(id: number, exe: string): ZoneDto {
  return {
    id,
    label: `Zone ${id}`,
    exe,
    path: '',
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
    w: 50,
    h: 100,
  };
}

describe('U-02 caller sizing classes win', () => {
  test('later caller classes override component defaults by Tailwind group', () => {
    expect(cn('h-9 w-full', 'h-8 w-52')).toBe('h-8 w-52');
  });

  test('non-conflicting classes are all preserved', () => {
    expect(cn('h-9 rounded-md', 'font-mono text-xs')).toBe('h-9 rounded-md font-mono text-xs');
  });

  test('falsy classes are dropped', () => {
    expect(cn('h-9', false, null, undefined, 'w-52')).toBe('h-9 w-52');
  });
});

describe('U-04 hotkey labels describe real behaviour', () => {
  test('profile_N is a zone launch, not a profile apply', () => {
    expect(hotkeyLabel('profile_1')).toBe('Launch/focus zone 1 (current profile)');
    expect(hotkeyLabel('profile_9')).toBe('Launch/focus zone 9 (current profile)');
    expect(hotkeyLabel('profile_1')).not.toContain('Apply profile');
  });

  test('friendly names and unknown names are unchanged', () => {
    expect(hotkeyLabel('clear_layout')).toBe('Clear layout');
    expect(hotkeyLabel('move_left')).toBe('Move window left');
    expect(hotkeyLabel('custom_binding')).toBe('custom_binding');
  });
});

describe('U-07 compact profile selector action mapping', () => {
  test('maps a profile name to the same load action as the rail', () => {
    expect(compactProfileLoadOptions('Trading')).toEqual({
      profile: 'Trading',
      action: 'switch to Trading',
      successMessage: 'Loaded Trading into the editor draft.',
    });
  });
});

describe('U-09 wizard replace assignment warning', () => {
  test('counts only assigned zones that will be deleted', () => {
    const zones = [
      zone(1, 'chrome.exe'), // kept — assignment preserved
      zone(2, 'code.exe'), // deleted + assigned
      zone(3, ''), // deleted but unassigned
      zone(4, 'firefox.exe'), // deleted + assigned
    ];
    expect(countAssignedZonesToRemove(zones, 1)).toBe(2);
  });

  test('returns zero when no deleted zone is assigned', () => {
    const zones = [zone(1, 'chrome.exe'), zone(2, ''), zone(3, '')];
    expect(countAssignedZonesToRemove(zones, 1)).toBe(0);
  });

  test('returns zero for a single zone (nothing deleted)', () => {
    expect(countAssignedZonesToRemove([zone(1, 'chrome.exe')], 1)).toBe(0);
  });
});

describe('B-3 / B-6 browse tab state', () => {
  test('a browse pick keeps the active tab (no reload overwrite)', () => {
    expect(sourceAfterBrowse('installed')).toBe('installed');
    expect(sourceAfterBrowse('running')).toBe('running');
  });

  test('entering Browse clears the list; other tabs load from the API', () => {
    expect(appsOnTabEnter('browse')).toEqual([]);
    expect(appsOnTabEnter('installed')).toBeNull();
    expect(appsOnTabEnter('running')).toBeNull();
  });
});
