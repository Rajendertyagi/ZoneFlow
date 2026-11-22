/**
 * Canvas geometry helpers — a faithful TypeScript port of the authoritative
 * rules in `src/engine_core.rs`.
 *
 * Zones are stored as normalized percentages (0..100) of the work area. The
 * SVG viewBox is the work area in pixels; conversion happens at the component
 * boundary via `getScreenCTM()`. Keeping the math here (pure, dependency-free)
 * makes it directly testable and auditable against the Rust source.
 */

import type { ZoneDto } from '../../api/client';

export interface RectPct {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface ZoneRect extends RectPct {
  id: number;
}

/** A shared divider between adjacent zones, in percent coordinates. */
export interface SharedBoundary {
  axis: 'v' | 'h';
  /** Position along the divider's own axis (percent). */
  at: number;
  /** Start of the span along the perpendicular axis (percent). */
  start: number;
  /** End of the span along the perpendicular axis (percent). */
  end: number;
}

export interface ResizeResult {
  /** The clamped divider position actually applied. */
  at: number;
  /** New geometry for every zone touching the divider, keyed by zone id. */
  changes: Record<number, RectPct>;
}

/** A shared divider handle: moves both neighbouring zones via `resizeBoundary`. */
export interface SharedResizeHandle {
  kind: 'shared';
  axis: 'v' | 'h';
  at: number;
  start: number;
  end: number;
  boundary: SharedBoundary;
}

/** An outer zone edge: resized alone via `resizeZoneEdge` (one undo point). */
export interface OuterResizeHandle {
  kind: 'outer';
  axis: 'v' | 'h';
  at: number;
  start: number;
  end: number;
  zoneId: number;
  /** 'low' = left/top edge, 'high' = right/bottom edge. */
  edge: 'low' | 'high';
}

/** A grabbable resize edge, shared divider or outer zone edge. */
export type ResizeHandle = SharedResizeHandle | OuterResizeHandle;

export function clamp(value: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, value));
}

/**
 * `engine_core::snap_percent`: snap `value` to the nearest point in `points`
 * within `tolerance`; values outside every tolerance window are unchanged.
 */
export function snapPercent(value: number, points: number[], tolerance: number): number {
  let best: { distance: number; point: number } | null = null;
  for (const point of points) {
    const distance = Math.abs(value - point);
    if (distance <= tolerance && (best === null || distance < best.distance)) {
      best = { distance, point };
    }
  }
  return best ? best.point : value;
}

/**
 * `engine_core::find_shared_boundaries`: a vertical boundary exists at `x`
 * when one zone's right edge and another zone's left edge are within
 * `tolerance` of `x`; its span covers the y-range of all touching zones.
 * Horizontal boundaries are the analogous case for top/bottom edges.
 */
export function findSharedBoundaries(zones: RectPct[], tolerance: number): SharedBoundary[] {
  const found: SharedBoundary[] = [];

  const same = (a: SharedBoundary, b: SharedBoundary): boolean =>
    a.axis === b.axis &&
    Math.abs(a.at - b.at) <= tolerance &&
    Math.abs(a.start - b.start) <= tolerance &&
    Math.abs(a.end - b.end) <= tolerance;

  const push = (candidate: SharedBoundary): void => {
    if (!found.some((existing) => same(existing, candidate))) found.push(candidate);
  };

  for (const z of zones) {
    // Vertical candidates: this zone's left and right edges.
    for (const at of [z.x, z.x + z.w]) {
      const before = zones.filter((o) => Math.abs(o.x + o.w - at) <= tolerance);
      const after = zones.filter((o) => Math.abs(o.x - at) <= tolerance);
      if (before.length === 0 || after.length === 0) continue;
      const touching = before.concat(after);
      push({
        axis: 'v',
        at,
        start: Math.min(...touching.map((o) => o.y)),
        end: Math.max(...touching.map((o) => o.y + o.h)),
      });
    }

    // Horizontal candidates: this zone's top and bottom edges.
    for (const at of [z.y, z.y + z.h]) {
      const above = zones.filter((o) => Math.abs(o.y + o.h - at) <= tolerance);
      const below = zones.filter((o) => Math.abs(o.y - at) <= tolerance);
      if (above.length === 0 || below.length === 0) continue;
      const touching = above.concat(below);
      push({
        axis: 'h',
        at,
        start: Math.min(...touching.map((o) => o.x)),
        end: Math.max(...touching.map((o) => o.x + o.w)),
      });
    }
  }

  return found;
}

/**
 * Outer-edge resize handles. Every zone edge that is NOT a shared divider gets
 * a handle, so a lone zone (zero shared boundaries) is still fully resizable.
 * This includes the work-area perimeter (x=0, x+w=100, y=0, y+h=100) and any
 * edge left unpaired by a gap — the common case is the perimeter.
 *
 * `findSharedBoundaries` is deliberately left untouched: it is a faithful port
 * of `engine_core::find_shared_boundaries` and its "shared only" contract is
 * covered by tests. Outer handles are a UI concern layered on top.
 */
export function zoneEdgeHandles(
  zones: ZoneRect[],
  boundaries: SharedBoundary[],
  tolerance: number,
): OuterResizeHandle[] {
  const handles: OuterResizeHandle[] = [];
  const isShared = (axis: 'v' | 'h', at: number, start: number, end: number): boolean =>
    boundaries.some(
      (b) =>
        b.axis === axis &&
        Math.abs(b.at - at) <= tolerance &&
        b.start < end - 1e-6 &&
        b.end > start + 1e-6,
    );

  for (const z of zones) {
    const vEdges: Array<{ at: number; edge: 'low' | 'high' }> = [
      { at: z.x, edge: 'low' },
      { at: z.x + z.w, edge: 'high' },
    ];
    for (const e of vEdges) {
      if (isShared('v', e.at, z.y, z.y + z.h)) continue;
      handles.push({
        kind: 'outer',
        axis: 'v',
        at: e.at,
        start: z.y,
        end: z.y + z.h,
        zoneId: z.id,
        edge: e.edge,
      });
    }

    const hEdges: Array<{ at: number; edge: 'low' | 'high' }> = [
      { at: z.y, edge: 'low' },
      { at: z.y + z.h, edge: 'high' },
    ];
    for (const e of hEdges) {
      if (isShared('h', e.at, z.x, z.x + z.w)) continue;
      handles.push({
        kind: 'outer',
        axis: 'h',
        at: e.at,
        start: z.x,
        end: z.x + z.w,
        zoneId: z.id,
        edge: e.edge,
      });
    }
  }
  return handles;
}

/**
 * Resize one zone's outer edge. The opposite edge stays fixed; the moved edge
 * is snapped to `snapPoints` and clamped to the 0..100 work area so the zone
 * never falls below `minPct`. Returns null only when the zone is already too
 * small to keep a `minPct` remainder (a degenerate case the UI never creates).
 */
export function resizeZoneEdge(
  zone: ZoneRect,
  axis: 'v' | 'h',
  edge: 'low' | 'high',
  requestedAt: number,
  minPct: number,
  snapPoints: number[],
  tolerance: number,
): RectPct | null {
  const at = snapPercent(requestedAt, snapPoints, tolerance);
  if (axis === 'v') {
    const right = zone.x + zone.w;
    if (edge === 'low') {
      const hi = right - minPct;
      if (hi < 0) return null;
      const x = clamp(at, 0, hi);
      return { x, y: zone.y, w: right - x, h: zone.h };
    }
    const lo = zone.x + minPct;
    if (lo > 100) return null;
    const newRight = clamp(at, lo, 100);
    return { x: zone.x, y: zone.y, w: newRight - zone.x, h: zone.h };
  }
  const bottom = zone.y + zone.h;
  if (edge === 'low') {
    const hi = bottom - minPct;
    if (hi < 0) return null;
    const y = clamp(at, 0, hi);
    return { x: zone.x, y, w: zone.w, h: bottom - y };
  }
  const lo = zone.y + minPct;
  if (lo > 100) return null;
  const newBottom = clamp(at, lo, 100);
  return { x: zone.x, y: zone.y, w: zone.w, h: newBottom - zone.y };
}

/**
 * Move a zone body, clamped to the work area. Geometry is unchanged, so the
 * minimum-zone rule is preserved automatically.
 */
export function moveZone(zone: RectPct, dx: number, dy: number): RectPct {
  return {
    x: clamp(zone.x + dx, 0, 100 - zone.w),
    y: clamp(zone.y + dy, 0, 100 - zone.h),
    w: zone.w,
    h: zone.h,
  };
}

/**
 * Drag a shared divider. Every zone on the low side keeps its start edge and
 * changes its extent; every zone on the high side keeps its end edge and moves
 * its start. The requested position is clamped so no zone falls below
 * `minPct` and nothing leaves the 0..100 work area.
 */
export function resizeBoundary(
  zones: ZoneRect[],
  boundary: SharedBoundary,
  requestedAt: number,
  minPct: number,
  tolerance: number,
): ResizeResult | null {
  const isV = boundary.axis === 'v';
  const low: ZoneRect[] = [];
  const high: ZoneRect[] = [];

  for (const z of zones) {
    const overlapsSpan = isV
      ? z.y < boundary.end - 1e-6 && z.y + z.h > boundary.start + 1e-6
      : z.x < boundary.end - 1e-6 && z.x + z.w > boundary.start + 1e-6;
    if (!overlapsSpan) continue;

    if (isV) {
      if (Math.abs(z.x + z.w - boundary.at) <= tolerance) low.push(z);
      if (Math.abs(z.x - boundary.at) <= tolerance) high.push(z);
    } else {
      if (Math.abs(z.y + z.h - boundary.at) <= tolerance) low.push(z);
      if (Math.abs(z.y - boundary.at) <= tolerance) high.push(z);
    }
  }

  if (low.length === 0 || high.length === 0) return null;

  let lo = 0;
  let hi = 100;
  for (const z of low) lo = Math.max(lo, (isV ? z.x : z.y) + minPct);
  for (const z of high) hi = Math.min(hi, (isV ? z.x + z.w : z.y + z.h) - minPct);
  if (lo > hi) return null;

  const at = clamp(requestedAt, lo, hi);
  const changes: Record<number, RectPct> = {};
  for (const z of low) {
    changes[z.id] = isV
      ? { x: z.x, y: z.y, w: at - z.x, h: z.h }
      : { x: z.x, y: z.y, w: z.w, h: at - z.y };
  }
  for (const z of high) {
    changes[z.id] = isV
      ? { x: at, y: z.y, w: z.x + z.w - at, h: z.h }
      : { x: z.x, y: at, w: z.w, h: z.y + z.h - at };
  }
  return { at, changes };
}

/** Axis-aligned rectangle from two pointer positions (percent coordinates). */
export function rectFromPoints(a: { x: number; y: number }, b: { x: number; y: number }): RectPct {
  return {
    x: Math.min(a.x, b.x),
    y: Math.min(a.y, b.y),
    w: Math.abs(b.x - a.x),
    h: Math.abs(b.y - a.y),
  };
}

/** Snap a drawn rectangle to the settings grid, enforcing the minimum size. */
export function snapRectToGrid(r: RectPct, cols: number, rows: number, minPct: number): RectPct {
  const cw = 100 / Math.max(1, cols);
  const ch = 100 / Math.max(1, rows);
  let x = Math.round(r.x / cw) * cw;
  let y = Math.round(r.y / ch) * ch;
  let right = Math.round((r.x + r.w) / cw) * cw;
  let bottom = Math.round((r.y + r.h) / ch) * ch;

  if (right - x < minPct) right = x + minPct;
  if (bottom - y < minPct) bottom = y + minPct;

  x = clamp(x, 0, 100);
  y = clamp(y, 0, 100);
  right = clamp(right, 0, 100);
  bottom = clamp(bottom, 0, 100);
  if (right - x < minPct) x = Math.max(0, right - minPct);
  if (bottom - y < minPct) y = Math.max(0, bottom - minPct);

  return { x, y, w: right - x, h: bottom - y };
}

/**
 * Clamp a split ratio so both halves respect `minPct`, while staying inside
 * the backend's own 0.05..0.95 clamp (`commands.rs::ZoneSplit`).
 */
export function clampSplitRatio(zone: RectPct, axis: 'x' | 'y', ratio: number, minPct: number): number {
  const extent = axis === 'x' ? zone.w : zone.h;
  if (extent <= 0) return 0.5;
  const lo = Math.max(0.05, minPct / extent);
  const hi = Math.min(0.95, 1 - minPct / extent);
  if (lo > hi) return 0.5;
  return clamp(ratio, lo, hi);
}

/** First grid cell that does not overlap an existing zone (for the "+" action). */
export function firstFreeCell(
  zones: RectPct[],
  cols: number,
  rows: number,
  minPct: number,
): RectPct {
  const cw = 100 / Math.max(1, cols);
  const ch = 100 / Math.max(1, rows);
  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) {
      let x = col * cw;
      let y = row * ch;
      let w = cw;
      let h = ch;
      if (w < minPct) {
        w = minPct;
        x = Math.min(x, 100 - w);
      }
      if (h < minPct) {
        h = minPct;
        y = Math.min(y, 100 - h);
      }
      const overlaps = zones.some(
        (z) => x < z.x + z.w && x + w > z.x && y < z.y + z.h && y + h > z.y,
      );
      if (!overlaps) return { x, y, w, h };
    }
  }
  return { x: 0, y: 0, w: 50, h: 50 };
}

/** A fully-formed ZoneDto with default rules and the supplied geometry. */
export function blankZone(rect: RectPct, label = 'New zone'): ZoneDto {
  return {
    id: 0,
    label,
    exe: '',
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
    // Match `Zone::default().key_delay` (config.rs) so new zones carry the
    // backend's real default and D3 confirmation never false-positives.
    key_delay: 80,
    x: rect.x,
    y: rect.y,
    w: rect.w,
    h: rect.h,
  };
}
