/**
 * Tiny module-level registry that lets the global keydown handler in
 * `App.svelte` reach the undo/redo functions owned by `ZoneCanvas.svelte`
 * without threading props through EditorView or adding a backend command.
 *
 * Plain `let` (not `$state`): the handler reads it imperatively inside an event
 * callback, so no reactivity is needed. The canvas registers on mount and
 * clears on destroy; the identity-checked unregister avoids a stale teardown
 * wiping out a newer registration.
 */

export interface CanvasCommands {
  undo: () => void;
  redo: () => void;
}

let current: CanvasCommands | null = null;

/** Register (or replace) the active editor canvas commands. Pass null to clear. */
export function registerCanvasCommands(commands: CanvasCommands | null): void {
  current = commands;
}

/** Clear only if `commands` is still the active registration (safe unmount). */
export function unregisterCanvasCommands(commands: CanvasCommands): void {
  if (current === commands) current = null;
}

/** The active commands, or null when no editor canvas is mounted. */
export function getCanvasCommands(): CanvasCommands | null {
  return current;
}
