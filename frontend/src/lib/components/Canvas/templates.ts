/**
 * Layout templates for the wizard. Geometry is expressed with the same
 * normalized `RectPct` type the canvas uses, so the live preview and the zones
 * actually created by `blankZone()` are guaranteed to match.
 */

import type { RectPct } from './geometry';

export type TemplateId = 'duo' | 'trio' | 'quad' | 'rail' | 'stack' | 'single' | 'grid';

export interface TemplateOption {
  id: TemplateId;
  label: string;
  hint: string;
}

export const TEMPLATE_OPTIONS: TemplateOption[] = [
  { id: 'duo', label: 'Duo', hint: 'Two vertical halves' },
  { id: 'trio', label: 'Trio', hint: 'Three columns' },
  { id: 'quad', label: 'Quad', hint: '2 × 2 grid' },
  { id: 'rail', label: 'Focus rail', hint: '30 / 70 split' },
  { id: 'stack', label: 'Stack', hint: 'Top / bottom' },
  { id: 'single', label: 'Single', hint: 'One full-area zone' },
  { id: 'grid', label: 'Custom grid', hint: 'Choose columns and rows' },
];

/** Generate the rectangles for a template. Pure — no API, no DOM. */
export function templateRects(id: TemplateId, cols: number, rows: number): RectPct[] {
  const c = Math.max(1, Math.min(12, Math.floor(cols) || 1));
  const r = Math.max(1, Math.min(12, Math.floor(rows) || 1));
  switch (id) {
    case 'duo':
      return [
        { x: 0, y: 0, w: 50, h: 100 },
        { x: 50, y: 0, w: 50, h: 100 },
      ];
    case 'trio':
      return [0, 1, 2].map((i) => ({ x: (100 / 3) * i, y: 0, w: 100 / 3, h: 100 }));
    case 'quad':
      return [0, 1].flatMap((row) =>
        [0, 1].map((col) => ({ x: col * 50, y: row * 50, w: 50, h: 50 })),
      );
    case 'rail':
      return [
        { x: 0, y: 0, w: 30, h: 100 },
        { x: 30, y: 0, w: 70, h: 100 },
      ];
    case 'stack':
      return [
        { x: 0, y: 0, w: 100, h: 50 },
        { x: 0, y: 50, w: 100, h: 50 },
      ];
    case 'single':
      return [{ x: 0, y: 0, w: 100, h: 100 }];
    case 'grid':
      return Array.from({ length: c * r }, (_, i) => ({
        x: (i % c) * (100 / c),
        y: Math.floor(i / c) * (100 / r),
        w: 100 / c,
        h: 100 / r,
      }));
  }
}

export function templateLabel(id: TemplateId, index: number): string {
  const base = TEMPLATE_OPTIONS.find((t) => t.id === id)?.label ?? 'Zone';
  return `${base} ${index + 1}`;
}
