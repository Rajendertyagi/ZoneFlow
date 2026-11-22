/**
 * Pure, rune-free reconciliation of the canvas selection after an undo/redo.
 *
 * Kept in its own module (rather than inside a Svelte component) so the exact
 * precedence can be unit-tested with bun — the repo has no component-test
 * framework. The caller passes the authoritative snapshot and its current local
 * selection; this function never reads or writes reactive state.
 */
export function reconcileSelection(
  snapshot: { zones: Array<{ id: number }>; selected_zone: number } | null,
  currentId: number | null,
): number | null {
  // No authoritative information: leave the local selection untouched.
  if (snapshot === null) return currentId;

  const exists = (id: number): boolean => snapshot.zones.some((z) => z.id === id);

  // A valid local selection is operational state — do not clobber it with the
  // backend's recorded selection just because an undo/redo happened.
  if (currentId !== null && exists(currentId)) return currentId;

  // The local selection is null or stale (e.g. the selected zone was deleted
  // locally before the undo). Restore the backend's recorded selection when it
  // is a real zone in the restored set; otherwise clear it so the inspector
  // never shows a zone that no longer exists.
  const recorded = snapshot.selected_zone;
  return recorded >= 0 && exists(recorded) ? recorded : null;
}
