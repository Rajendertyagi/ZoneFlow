/**
 * WP3 — pure keyboard/focus helpers for the modernized UI primitives.
 *
 * These are the DOM-free parts of the dialog focus trap, the context-menu
 * roving focus and the tabs arrow navigation. DOM-level component tests are a
 * documented limitation: this repo has no DOM test harness and WP3 adds no new
 * dependencies, so the .svelte components own only element wiring while the
 * decision math is covered here with `bun test`.
 */
import { describe, expect, test } from 'bun:test';
import {
  edgeEnabledIndex,
  edgeIndex,
  enabledIndices,
  FOCUSABLE_SELECTOR,
  isDismissKey,
  nextEnabledIndex,
  rovingStep,
  trapTabIndex,
  wrapIndex,
} from '../src/lib/components/ui/a11y-helpers';

describe('wrapIndex / edgeIndex / rovingStep', () => {
  test('wraps in both directions', () => {
    expect(wrapIndex(4, 4)).toBe(0);
    expect(wrapIndex(-1, 4)).toBe(3);
    expect(wrapIndex(9, 4)).toBe(1);
  });

  test('returns -1 for an empty collection', () => {
    expect(wrapIndex(0, 0)).toBe(-1);
    expect(edgeIndex('home', 0)).toBe(-1);
    expect(edgeIndex('end', 0)).toBe(-1);
    expect(rovingStep(0, 0, true)).toBe(-1);
  });

  test('Home/End map to the first/last index', () => {
    expect(edgeIndex('home', 4)).toBe(0);
    expect(edgeIndex('end', 4)).toBe(3);
  });

  test('tabs arrow navigation wraps at both ends', () => {
    expect(rovingStep(0, 3, false)).toBe(2); // ArrowLeft from first -> last
    expect(rovingStep(2, 3, true)).toBe(0); // ArrowRight from last -> first
    expect(rovingStep(1, 3, true)).toBe(2);
    expect(rovingStep(1, 3, false)).toBe(0);
  });
});

describe('trapTabIndex (dialog focus trap)', () => {
  test('wraps Tab on the last element and Shift+Tab on the first', () => {
    expect(trapTabIndex(2, 3, false)).toBe(0);
    expect(trapTabIndex(0, 3, true)).toBe(2);
  });

  test('leaves normal Tab handling alone in the middle', () => {
    expect(trapTabIndex(1, 3, false)).toBeNull();
    expect(trapTabIndex(1, 3, true)).toBeNull();
  });

  test('pulls focus back when it is outside the dialog', () => {
    expect(trapTabIndex(-1, 3, false)).toBe(0);
    expect(trapTabIndex(-1, 3, true)).toBe(2);
    expect(trapTabIndex(99, 3, false)).toBe(0);
  });

  test('no-op when there is nothing focusable', () => {
    expect(trapTabIndex(-1, 0, false)).toBeNull();
  });
});

describe('FOCUSABLE_SELECTOR', () => {
  test('covers native controls and excludes disabled ones / tabindex=-1', () => {
    expect(FOCUSABLE_SELECTOR).toContain('button:not([disabled])');
    expect(FOCUSABLE_SELECTOR).toContain('input:not([disabled])');
    expect(FOCUSABLE_SELECTOR).toContain('[tabindex]:not([tabindex="-1"])');
  });
});

describe('context-menu roving focus over enabled items', () => {
  const items = [
    { label: 'a' },
    { label: 'b', disabled: true },
    { label: 'c' },
    { label: 'd' },
  ];

  test('enabledIndices skips disabled items', () => {
    expect(enabledIndices(items)).toEqual([0, 2, 3]);
    expect(enabledIndices([{ label: 'x', disabled: true }])).toEqual([]);
  });

  test('ArrowDown/ArrowUp skip disabled items and wrap', () => {
    expect(nextEnabledIndex(items, 0, true)).toBe(2); // a -> c (skip b)
    expect(nextEnabledIndex(items, 2, false)).toBe(0); // c -> a (skip b)
    expect(nextEnabledIndex(items, 3, true)).toBe(0); // d -> a (wrap)
    expect(nextEnabledIndex(items, 0, false)).toBe(3); // a -> d (wrap)
  });

  test('opening focuses the first enabled item', () => {
    expect(nextEnabledIndex(items, -1, true)).toBe(0);
  });

  test('Home/End land on the first/last enabled item', () => {
    expect(edgeEnabledIndex(items, 'home')).toBe(0);
    expect(edgeEnabledIndex(items, 'end')).toBe(3);
    expect(edgeEnabledIndex([{ label: 'x', disabled: true }], 'home')).toBe(-1);
  });
});

describe('isDismissKey', () => {
  test('only Escape dismisses transient overlays', () => {
    expect(isDismissKey('Escape')).toBe(true);
    expect(isDismissKey('Enter')).toBe(false);
    expect(isDismissKey('Tab')).toBe(false);
  });
});
