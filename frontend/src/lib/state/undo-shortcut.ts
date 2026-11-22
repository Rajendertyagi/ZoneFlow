/**
 * Pure decision logic for the editor's undo/redo keyboard shortcuts.
 *
 * No Svelte runes and no DOM globals: the helpers operate on structural
 * event/target shapes so they can be unit-tested with plain objects under bun.
 * The Svelte side (App.svelte) supplies the real KeyboardEvent plus the current
 * view / dialog / history state, then dispatches the returned action to the
 * canvas command registry.
 */

export type UndoShortcutAction = 'undo' | 'redo';

/** The four views the SPA can show (kept local so this module stays rune-free). */
export type EditorViewName = 'home' | 'editor' | 'hotkeys' | 'settings';

/** Minimal structural view of the KeyboardEvent fields the decision uses. */
export interface UndoShortcutEvent {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  /** The real `event.target`; duck-typed by `isEditableTarget`. */
  target: unknown;
}

/** Everything else the decision depends on, supplied by the caller. */
export interface UndoShortcutContext {
  view: EditorViewName;
  canUndo: boolean;
  canRedo: boolean;
  /** True when any modal dialog is open. */
  dialogOpen: boolean;
}

const EDITABLE_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT']);
// Match only genuinely editable contenteditable values, not `contenteditable="false"`.
const CONTENTEDITABLE_SELECTOR =
  'input, textarea, select, [contenteditable=""], [contenteditable="true"], [contenteditable="plaintext-only"]';

/**
 * True when a keydown target is a text-entry element (or lives inside one).
 * Duck-typed on purpose: no `instanceof HTMLElement`, so bun can test it with
 * plain objects and the real DOM still works via `tagName` / `isContentEditable`
 * / `closest`.
 */
export function isEditableTarget(target: unknown): boolean {
  if (!target || typeof target !== 'object') return false;
  const el = target as {
    tagName?: string;
    isContentEditable?: boolean;
    closest?: (selector: string) => unknown;
  };
  if (typeof el.tagName === 'string' && EDITABLE_TAGS.has(el.tagName.toUpperCase())) return true;
  if (el.isContentEditable === true) return true;
  if (typeof el.closest === 'function' && el.closest(CONTENTEDITABLE_SELECTOR) != null) return true;
  return false;
}

/**
 * Classify a modifier combo as undo / redo / neither, without any gating.
 *
 * Supported combos (matching the README):
 *   - Ctrl+Z / Cmd+Z            -> undo
 *   - Ctrl+Shift+Z / Cmd+Shift+Z -> redo
 *   - Ctrl+Y                    -> redo
 * Alt and mixed Ctrl+Meta combos are deliberately ignored, as is Cmd+Y.
 */
export function classifyUndoShortcut(
  event: Pick<UndoShortcutEvent, 'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>,
): UndoShortcutAction | null {
  const key = typeof event.key === 'string' ? event.key.toLowerCase() : '';
  if (event.altKey || (event.ctrlKey && event.metaKey)) return null;
  if (!event.ctrlKey && !event.metaKey) return null;

  if (key === 'z') return event.shiftKey ? 'redo' : 'undo';
  // Ctrl+Y only; Cmd+Y is not a documented shortcut.
  if (key === 'y' && event.ctrlKey && !event.metaKey && !event.shiftKey) return 'redo';
  return null;
}

/**
 * Full pure decision: which action (if any) the global handler should dispatch.
 * Returns null when typing, in a dialog, outside the editor, or when the
 * corresponding history action is unavailable.
 */
export function resolveUndoShortcut(
  event: UndoShortcutEvent,
  context: UndoShortcutContext,
): UndoShortcutAction | null {
  if (context.view !== 'editor') return null;
  if (context.dialogOpen) return null;
  if (isEditableTarget(event.target)) return null;

  const action = classifyUndoShortcut(event);
  if (action === null) return null;
  if (action === 'undo' && !context.canUndo) return null;
  if (action === 'redo' && !context.canRedo) return null;
  return action;
}

/** Boolean convenience wrapper around `resolveUndoShortcut`. */
export function shouldHandleUndoShortcut(
  event: UndoShortcutEvent,
  context: UndoShortcutContext,
): boolean {
  return resolveUndoShortcut(event, context) !== null;
}
