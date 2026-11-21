/**
 * Geometry regression tests — run with `bun test tests/geometry.test.ts`.
 *
 * These mirror the Rust unit tests in `src/engine_core.rs` so the frontend's
 * port cannot silently drift from the authoritative geometry rules. Bun's
 * built-in runner needs no new dependency.
 */
import { describe, expect, test } from 'bun:test';
import {
  clampSplitRatio,
  findSharedBoundaries,
  firstFreeCell,
  moveZone,
  rectFromPoints,
  resizeBoundary,
  snapPercent,
  snapRectToGrid,
  type ZoneRect,
} from '../src/lib/components/Canvas/geometry';

function zone(id: number, x: number, y: number, w: number, h: number): ZoneRect {
  return { id, x, y, w, h };
}

describe('snapPercent (engine_core::snap_percent)', () => {
  test('snaps to 25/50/75 within tolerance', () => {
    expect(snapPercent(24.2, [25, 50, 75], 1.5)).toBe(25);
    expect(snapPercent(51, [25, 50, 75], 1.5)).toBe(50);
    expect(snapPercent(74, [25, 50, 75], 1.5)).toBe(75);
  });

  test('leaves values outside every tolerance window unchanged', () => {
    expect(snapPercent(40, [25, 50, 75], 1.5)).toBe(40);
    expect(snapPercent(50, [25, 50, 75], 0)).toBe(50);
  });
});

describe('findSharedBoundaries (engine_core::find_shared_boundaries)', () => {
  test('detects a single vertical divider', () => {
    const boundaries = findSharedBoundaries([zone(1, 0, 0, 50, 100), zone(2, 50, 0, 50, 100)], 1.5);
    expect(boundaries).toEqual([{ axis: 'v', at: 50, start: 0, end: 100 }]);
  });

  test('detects both dividers in a 2x2 grid', () => {
    const boundaries = findSharedBoundaries(
      [zone(1, 0, 0, 50, 50), zone(2, 50, 0, 50, 50), zone(3, 0, 50, 50, 50), zone(4, 50, 50, 50, 50)],
      1.5,
    );
    expect(boundaries).toContainEqual({ axis: 'v', at: 50, start: 0, end: 100 });
    expect(boundaries).toContainEqual({ axis: 'h', at: 50, start: 0, end: 100 });
  });

  test('tolerates a small gap between zones', () => {
    const boundaries = findSharedBoundaries([zone(1, 0, 0, 49.5, 100), zone(2, 50, 0, 50, 100)], 1.5);
    expect(boundaries.length).toBe(1);
    expect(boundaries[0].at).toBe(49.5);
  });

  test('does not report outer edges as shared', () => {
    const boundaries = findSharedBoundaries([zone(1, 0, 0, 100, 100)], 1.5);
    expect(boundaries).toEqual([]);
  });
});

describe('resizeBoundary', () => {
  const zones = [zone(1, 0, 0, 50, 100), zone(2, 50, 0, 50, 100)];
  const boundary = { axis: 'v' as const, at: 50, start: 0, end: 100 };

  test('moves both sides of a divider together', () => {
    const result = resizeBoundary(zones, boundary, 60, 15, 1.5);
    expect(result).not.toBeNull();
    expect(result!.at).toBe(60);
    expect(result!.changes[1]).toEqual({ x: 0, y: 0, w: 60, h: 100 });
    expect(result!.changes[2]).toEqual({ x: 60, y: 0, w: 40, h: 100 });
  });

  test('clamps so neither zone falls below min_zone_pct', () => {
    const result = resizeBoundary(zones, boundary, 95, 15, 1.5);
    expect(result!.at).toBe(85); // right zone keeps 15%
    expect(result!.changes[2].w).toBe(15);

    const low = resizeBoundary(zones, boundary, 5, 15, 1.5);
    expect(low!.at).toBe(15); // left zone keeps 15%
    expect(low!.changes[1].w).toBe(15);
  });

  test('snapped position is clamped after snapping', () => {
    const snapped = snapPercent(86, [25, 50, 75], 1.5); // 86 -> unchanged
    const result = resizeBoundary(zones, boundary, snapped, 15, 1.5);
    expect(result!.at).toBe(85);
  });
});

describe('moveZone', () => {
  test('clamps to the work area', () => {
    expect(moveZone({ x: 40, y: 40, w: 50, h: 50 }, 30, 30)).toEqual({ x: 50, y: 50, w: 50, h: 50 });
    expect(moveZone({ x: 10, y: 10, w: 50, h: 50 }, -30, -30)).toEqual({ x: 0, y: 0, w: 50, h: 50 });
  });
});

describe('snapRectToGrid', () => {
  test('snaps edges to grid lines and enforces the minimum', () => {
    const rect = rectFromPoints({ x: 9, y: 9 }, { x: 20, y: 20 });
    const snapped = snapRectToGrid(rect, 12, 8, 15);
    // 100/12 = 8.33, 100/8 = 12.5; 9 -> 8.33, 20 -> 16.67 (too small), extended to 15.
    expect(snapped.x).toBeCloseTo(100 / 12, 5);
    expect(snapped.y).toBeCloseTo(12.5, 5);
    expect(snapped.w).toBeGreaterThanOrEqual(15);
    expect(snapped.h).toBeGreaterThanOrEqual(15);
  });
});

describe('clampSplitRatio', () => {
  test('keeps both halves above min_zone_pct', () => {
    const zone = { x: 0, y: 0, w: 40, h: 40 };
    expect(clampSplitRatio(zone, 'x', 0.5, 15)).toBeCloseTo(0.5, 5);
    expect(clampSplitRatio(zone, 'x', 0.01, 15)).toBeCloseTo(0.375, 5); // 15/40
    expect(clampSplitRatio(zone, 'x', 0.99, 15)).toBeCloseTo(0.625, 5); // 1 - 15/40
  });

  test('respects the backend 0.05..0.95 clamp', () => {
    const wide = { x: 0, y: 0, w: 100, h: 100 };
    expect(clampSplitRatio(wide, 'x', 0.01, 15)).toBe(0.15);
    expect(clampSplitRatio(wide, 'x', 0.99, 15)).toBe(0.85);
  });
});

describe('firstFreeCell', () => {
  test('finds a non-overlapping grid cell', () => {
    const cell = firstFreeCell([zone(1, 0, 0, 50, 100)], 12, 8, 15);
    expect(cell.x).toBeGreaterThanOrEqual(50);
    expect(cell.x + cell.w).toBeLessThanOrEqual(100);
  });
});
