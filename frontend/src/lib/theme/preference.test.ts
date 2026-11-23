/// <reference types="node" />
// @ts-ignore - bun:test types are provided by the Bun runtime, not @types/node.
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  DEFAULT_APPEARANCE,
  DEFAULT_MODE,
  MODE_STORAGE_KEY,
  THEME_STORAGE_KEY,
  isValidMode,
  isValidPresetId,
  normalizeAppearance,
  parseMode,
  parsePresetId,
  resolveMode,
} from './preference';
import { DEFAULT_PRESET_ID, PRESET_IDS } from '../../themes/manifest';

describe('ModeWatcher storage keys', () => {
  test('match the documented ModeWatcher keys', () => {
    expect(MODE_STORAGE_KEY).toBe('mode-watcher-mode');
    expect(THEME_STORAGE_KEY).toBe('mode-watcher-theme');
  });
});

describe('mode validation', () => {
  test('accepts the three valid modes', () => {
    for (const mode of ['light', 'dark', 'system']) {
      expect(isValidMode(mode)).toBe(true);
      expect(parseMode(mode)).toBe(mode);
    }
  });

  test('falls back to system for invalid or missing values', () => {
    const invalid = ['', 'auto', 'LIGHT', 'dark ', 0, 1, null, undefined, {}, []];
    for (const value of invalid) {
      expect(isValidMode(value)).toBe(false);
      expect(parseMode(value)).toBe('system');
    }
    expect(DEFAULT_MODE).toBe('system');
  });
});

describe('preset validation', () => {
  test('accepts every built-in preset id', () => {
    for (const id of PRESET_IDS) {
      expect(isValidPresetId(id)).toBe(true);
      expect(parsePresetId(id)).toBe(id);
    }
  });

  test('falls back to zoneflow-amber for invalid or missing values', () => {
    const invalid = ['', 'bogus', 'ZoneFlow-Amber', 42, null, undefined, {}, []];
    for (const value of invalid) {
      expect(isValidPresetId(value)).toBe(false);
      expect(parsePresetId(value)).toBe('zoneflow-amber');
    }
    expect(DEFAULT_PRESET_ID).toBe('zoneflow-amber');
  });
});

describe('normalizeAppearance', () => {
  test('normalizes a valid pair', () => {
    expect(normalizeAppearance('dark', 'caffeine')).toEqual({
      mode: 'dark',
      presetId: 'caffeine',
    });
  });

  test('normalizes a fully invalid pair to defaults', () => {
    expect(normalizeAppearance('nonsense', 'nope')).toEqual(DEFAULT_APPEARANCE);
  });

  test('normalizes each field independently', () => {
    expect(normalizeAppearance('light', 'nope')).toEqual({ mode: 'light', presetId: 'zoneflow-amber' });
    expect(normalizeAppearance('nonsense', 'caffeine')).toEqual({ mode: 'system', presetId: 'caffeine' });
  });
});

describe('resolveMode', () => {
  test('resolves system using the OS preference', () => {
    expect(resolveMode('system', true)).toBe('dark');
    expect(resolveMode('system', false)).toBe('light');
  });

  test('keeps explicit modes regardless of OS preference', () => {
    expect(resolveMode('light', true)).toBe('light');
    expect(resolveMode('dark', false)).toBe('dark');
  });
});

describe('index.html pre-paint bootstrap', () => {
  const here = dirname(fileURLToPath(import.meta.url));
  const html = readFileSync(resolve(here, '..', '..', '..', 'index.html'), 'utf8');

  test('is the single pre-paint owner and injects no competing script', () => {
    // Exactly one inline (non-module) script before the module script.
    const inlineScripts = html.match(/<script(?![^>]*type="module")[^>]*>/g) ?? [];
    expect(inlineScripts).toHaveLength(1);
    expect(html.indexOf('<script>')).toBeLessThan(html.indexOf('type="module"'));
    expect(html).toContain("root.classList.toggle('dark'");
    expect(html).toContain('root.style.colorScheme = resolved');
    expect(html).toContain('root.dataset.theme = preset');
    expect(html).toContain("matchMedia('(prefers-color-scheme: dark)')");
  });

  test('uses the same keys, defaults and preset-id allowlist as the helpers', () => {
    expect(html).toContain(`'${MODE_STORAGE_KEY}'`);
    expect(html).toContain(`'${THEME_STORAGE_KEY}'`);
    expect(html).toContain(`'${DEFAULT_MODE}'`);
    expect(html).toContain(`'${DEFAULT_PRESET_ID}'`);

    // Exact parity: extract the bootstrap's PRESET_IDS array literal and
    // deep-equal it to the manifest ids, preserving order. A stale extra or
    // missing id in either place must fail this test.
    const match = html.match(/var\s+PRESET_IDS\s*=\s*\[([^\]]*)\]/);
    expect(match).not.toBeNull();
    const bootstrapIds = (match?.[1] ?? '')
      .split(',')
      .map((part) => part.trim().replace(/^['"]|['"]$/g, ''))
      .filter((part) => part.length > 0);
    expect(bootstrapIds).toEqual([...PRESET_IDS]);
  });

  test('writes normalized values back to both storage keys', () => {
    // ModeWatcher's onMount re-reads raw localStorage values, so the bootstrap
    // must persist the validated mode/preset to keep them in sync.
    expect(html).toMatch(/setItem\(\s*MODE_KEY\s*,\s*mode\s*\)/);
    expect(html).toMatch(/setItem\(\s*THEME_KEY\s*,\s*preset\s*\)/);
  });

  test('does not duplicate preset token values', () => {
    // The bootstrap must contain preset IDs only, never token values.
    expect(html).not.toContain('--background');
    expect(html).not.toContain('--canvas-bg');
    expect(html).not.toContain('#f59e0b');
  });
});
