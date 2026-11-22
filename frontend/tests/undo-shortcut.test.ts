/**
 * Tests for the pure undo/redo shortcut decision logic in `undo-shortcut.ts`.
 * The module is rune-free and DOM-global-free, so bun can exercise it with
 * plain objects — no component-test framework needed.
 */
import { describe, expect, test } from 'bun:test';
import {
  classifyUndoShortcut,
  isEditableTarget,
  resolveUndoShortcut,
  shouldHandleUndoShortcut,
  type UndoShortcutContext,
  type UndoShortcutEvent,
} from '../src/lib/state/undo-shortcut';

function event(overrides: Partial<UndoShortcutEvent> = {}): UndoShortcutEvent {
  return {
    key: 'z',
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    altKey: false,
    target: null,
    ...overrides,
  };
}

function context(overrides: Partial<UndoShortcutContext> = {}): UndoShortcutContext {
  return {
    view: 'editor',
    canUndo: true,
    canRedo: true,
    dialogOpen: false,
    ...overrides,
  };
}

describe('classifyUndoShortcut', () => {
  test('Ctrl+Z and Cmd+Z classify as undo', () => {
    expect(classifyUndoShortcut(event({ ctrlKey: true }))).toBe('undo');
    expect(classifyUndoShortcut(event({ metaKey: true }))).toBe('undo');
  });

  test('Ctrl+Shift+Z and Cmd+Shift+Z classify as redo', () => {
    expect(classifyUndoShortcut(event({ ctrlKey: true, shiftKey: true }))).toBe('redo');
    expect(classifyUndoShortcut(event({ metaKey: true, shiftKey: true }))).toBe('redo');
  });

  test('Ctrl+Y classifies as redo', () => {
    expect(classifyUndoShortcut(event({ key: 'y', ctrlKey: true }))).toBe('redo');
  });

  test('uppercase key (Shift+Z) is normalized', () => {
    expect(classifyUndoShortcut(event({ key: 'Z', ctrlKey: true, shiftKey: true }))).toBe('redo');
    expect(classifyUndoShortcut(event({ key: 'Z', ctrlKey: true }))).toBe('undo');
  });

  test('no primary modifier is not a shortcut', () => {
    expect(classifyUndoShortcut(event())).toBeNull();
    expect(classifyUndoShortcut(event({ shiftKey: true }))).toBeNull();
  });

  test('Alt and mixed Ctrl+Meta combos are ignored', () => {
    expect(classifyUndoShortcut(event({ altKey: true }))).toBeNull();
    expect(classifyUndoShortcut(event({ ctrlKey: true, altKey: true }))).toBeNull();
    expect(classifyUndoShortcut(event({ ctrlKey: true, metaKey: true }))).toBeNull();
  });

  test('Cmd+Y and Ctrl+Shift+Y are not supported', () => {
    expect(classifyUndoShortcut(event({ key: 'y', metaKey: true }))).toBeNull();
    expect(classifyUndoShortcut(event({ key: 'y', ctrlKey: true, shiftKey: true }))).toBeNull();
  });

  test('unrelated keys are ignored', () => {
    expect(classifyUndoShortcut(event({ key: 'x', ctrlKey: true }))).toBeNull();
    expect(classifyUndoShortcut(event({ key: '', ctrlKey: true }))).toBeNull();
  });
});

describe('isEditableTarget', () => {
  test('null and non-object targets are not editable', () => {
    expect(isEditableTarget(null)).toBe(false);
    expect(isEditableTarget(undefined)).toBe(false);
    expect(isEditableTarget('input')).toBe(false);
    expect(isEditableTarget({})).toBe(false);
  });

  test('input, textarea and select tags are editable (case-insensitive)', () => {
    expect(isEditableTarget({ tagName: 'INPUT' })).toBe(true);
    expect(isEditableTarget({ tagName: 'textarea' })).toBe(true);
    expect(isEditableTarget({ tagName: 'Select' })).toBe(true);
  });

  test('contenteditable elements are editable', () => {
    expect(isEditableTarget({ tagName: 'DIV', isContentEditable: true })).toBe(true);
  });

  test('a descendant of an editable element is editable via closest()', () => {
    let seenSelector = '';
    const target = {
      tagName: 'SPAN',
      closest: (selector: string) => {
        seenSelector = selector;
        return { tagName: 'DIV' };
      },
    };
    expect(isEditableTarget(target)).toBe(true);
    expect(seenSelector).toContain('contenteditable');
  });

  test('a non-editable element with a non-matching closest() is not editable', () => {
    expect(isEditableTarget({ tagName: 'BUTTON', closest: () => null })).toBe(false);
    expect(isEditableTarget({ tagName: 'DIV', isContentEditable: false })).toBe(false);
  });
});

describe('resolveUndoShortcut', () => {
  test('returns the correct action for each supported combo in the editor', () => {
    expect(resolveUndoShortcut(event({ ctrlKey: true }), context())).toBe('undo');
    expect(resolveUndoShortcut(event({ metaKey: true }), context())).toBe('undo');
    expect(resolveUndoShortcut(event({ ctrlKey: true, shiftKey: true }), context())).toBe('redo');
    expect(resolveUndoShortcut(event({ metaKey: true, shiftKey: true }), context())).toBe('redo');
    expect(resolveUndoShortcut(event({ key: 'y', ctrlKey: true }), context())).toBe('redo');
  });

  test('view gating: only the editor view handles shortcuts', () => {
    const undo = event({ ctrlKey: true });
    expect(resolveUndoShortcut(undo, context({ view: 'home' }))).toBeNull();
    expect(resolveUndoShortcut(undo, context({ view: 'hotkeys' }))).toBeNull();
    expect(resolveUndoShortcut(undo, context({ view: 'settings' }))).toBeNull();
    expect(resolveUndoShortcut(undo, context({ view: 'editor' }))).toBe('undo');
  });

  test('dialog suppression: an open dialog blocks every combo', () => {
    expect(resolveUndoShortcut(event({ ctrlKey: true }), context({ dialogOpen: true }))).toBeNull();
    expect(
      resolveUndoShortcut(event({ ctrlKey: true, shiftKey: true }), context({ dialogOpen: true })),
    ).toBeNull();
  });

  test('typing suppression: editable targets block every combo', () => {
    expect(resolveUndoShortcut(event({ ctrlKey: true, target: { tagName: 'INPUT' } }), context())).toBeNull();
    expect(
      resolveUndoShortcut(event({ key: 'y', ctrlKey: true, target: { tagName: 'TEXTAREA' } }), context()),
    ).toBeNull();
    expect(
      resolveUndoShortcut(event({ ctrlKey: true, target: { isContentEditable: true } }), context()),
    ).toBeNull();
  });

  test('can_undo gating: undo is suppressed when unavailable', () => {
    expect(resolveUndoShortcut(event({ ctrlKey: true }), context({ canUndo: false }))).toBeNull();
    // A disabled undo must not fall through to redo even when redo is available.
    expect(
      resolveUndoShortcut(event({ ctrlKey: true }), context({ canUndo: false, canRedo: true })),
    ).toBeNull();
    expect(resolveUndoShortcut(event({ ctrlKey: true }), context({ canUndo: true }))).toBe('undo');
  });

  test('can_redo gating: redo is suppressed when unavailable', () => {
    expect(
      resolveUndoShortcut(event({ ctrlKey: true, shiftKey: true }), context({ canRedo: false })),
    ).toBeNull();
    expect(
      resolveUndoShortcut(event({ key: 'y', ctrlKey: true }), context({ canRedo: false })),
    ).toBeNull();
    expect(
      resolveUndoShortcut(event({ ctrlKey: true, shiftKey: true }), context({ canRedo: true })),
    ).toBe('redo');
  });

  test('gating is independent: canRedo does not enable undo and vice versa', () => {
    expect(
      resolveUndoShortcut(event({ ctrlKey: true }), context({ canUndo: false, canRedo: true })),
    ).toBeNull();
    expect(
      resolveUndoShortcut(event({ key: 'y', ctrlKey: true }), context({ canUndo: true, canRedo: false })),
    ).toBeNull();
  });

  test('unsupported combos stay null even with all gates open', () => {
    expect(resolveUndoShortcut(event({ altKey: true, ctrlKey: true }), context())).toBeNull();
    expect(resolveUndoShortcut(event({ key: 'y', metaKey: true }), context())).toBeNull();
  });
});

describe('shouldHandleUndoShortcut', () => {
  test('is true only when a gated action is available', () => {
    expect(shouldHandleUndoShortcut(event({ ctrlKey: true }), context())).toBe(true);
    expect(shouldHandleUndoShortcut(event({ ctrlKey: true }), context({ canUndo: false }))).toBe(false);
    expect(shouldHandleUndoShortcut(event({ ctrlKey: true }), context({ view: 'home' }))).toBe(false);
    expect(shouldHandleUndoShortcut(event({ ctrlKey: true }), context({ dialogOpen: true }))).toBe(false);
  });
});
