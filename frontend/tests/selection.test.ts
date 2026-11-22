/**
 * Regression tests for `reconcileSelection` — the canvas selection must survive
 * undo/redo without clobbering a valid local selection, and must be restored
 * when the local selection was cleared or went stale.
 */
import { describe, expect, test } from 'bun:test';
import { reconcileSelection } from '../src/lib/state/selection';

function snap(ids: number[], selected_zone: number) {
  return { zones: ids.map((id) => ({ id })), selected_zone };
}

describe('reconcileSelection', () => {
  test('restores backend selection when currentId is null (delete -> undo)', () => {
    // The zone was deleted locally (selectedId = null); undo restores it and
    // the backend records it as selected again.
    expect(reconcileSelection(snap([1, 2, 3], 2), null)).toBe(2);
  });

  test('restores backend selection when currentId is stale', () => {
    // currentId points at a zone that no longer exists in the restored set.
    expect(reconcileSelection(snap([1, 2, 3], 3), 99)).toBe(3);
  });

  test('keeps a valid currentId even when the backend selected_zone differs', () => {
    // A valid local selection is operational state and must not be rewound.
    expect(reconcileSelection(snap([1, 2, 3], 3), 1)).toBe(1);
  });

  test('returns null when currentId is null and backend selection is invalid', () => {
    expect(reconcileSelection(snap([1, 2, 3], -1), null)).toBeNull();
    expect(reconcileSelection(snap([1, 2, 3], 99), null)).toBeNull();
  });

  test('returns null when currentId is stale and backend selection is invalid', () => {
    expect(reconcileSelection(snap([1, 2, 3], -1), 99)).toBeNull();
  });

  test('returns currentId unchanged when the snapshot is null', () => {
    expect(reconcileSelection(null, null)).toBeNull();
    expect(reconcileSelection(null, 7)).toBe(7);
  });
});
