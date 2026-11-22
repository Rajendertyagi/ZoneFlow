/**
 * Pure tests for the persistent hotkey-registration warning (Finding U-03).
 * `hotkey-failures.ts` has no Svelte runes, so bun can import it directly.
 */
import { describe, expect, test } from 'bun:test';
import { hotkeyFailureWarning } from '../src/lib/state/hotkey-failures';

describe('hotkeyFailureWarning', () => {
  test('no failures (or a missing snapshot field) renders no warning', () => {
    expect(hotkeyFailureWarning(undefined)).toBeNull();
    expect(hotkeyFailureWarning([])).toBeNull();
  });

  test('a single failure renders a persistent warning naming the binding', () => {
    const warning = hotkeyFailureWarning(['clear_layout (Ctr+Shift+0)']);
    expect(warning).not.toBeNull();
    expect(warning).toContain('These hotkeys could not be registered');
    expect(warning).toContain('clear_layout (Ctr+Shift+0)');
  });

  test('multiple failures are all listed', () => {
    const warning = hotkeyFailureWarning([
      'clear_layout (Ctr+Shift+0)',
      'move_left (Ctrl+Alt+Left)',
    ]);
    expect(warning).toContain('clear_layout (Ctr+Shift+0)');
    expect(warning).toContain('move_left (Ctrl+Alt+Left)');
  });
});
