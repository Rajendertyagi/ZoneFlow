// @ts-ignore - bun:test types are provided by the Bun runtime, not @types/node.
import { describe, expect, test } from 'bun:test';
import amberFixture from './presets/zoneflow-amber.json';
import caffeineFixture from './presets/caffeine.json';
import {
  DEFAULT_PRESET_ID,
  PRESET_IDS,
  PRESET_MANIFEST,
  getPreset,
  isKnownPresetId,
  resolvePresetId,
} from './manifest';
import {
  ALLOWED_THEME_KEYS,
  LIGHT_DARK_KEYS,
  REQUIRED_LIGHT_DARK_KEYS,
  SHARED_THEME_KEYS,
  ThemePresetError,
  isValidColorValue,
  normalizePreset,
  validatePreset,
} from './schema';

const FIXTURES = [
  ['zoneflow-amber', amberFixture],
  ['caffeine', caffeineFixture],
] as const;

function clone<T>(value: T): T {
  return structuredClone(value);
}

describe('built-in preset fixtures', () => {
  test('manifest ids match fixture names and default is zoneflow-amber', () => {
    expect(DEFAULT_PRESET_ID).toBe('zoneflow-amber');
    expect(PRESET_IDS).toEqual(['zoneflow-amber', 'caffeine']);
    for (const [id, fixture] of FIXTURES) {
      expect(fixture.name).toBe(id);
      expect(fixture.type).toBe('registry:style');
    }
  });

  test('every fixture validates with all required tokens in both modes', () => {
    for (const [id, fixture] of FIXTURES) {
      const result = validatePreset(fixture);
      expect(result.ok, `${id}: ${result.ok ? '' : result.error}`).toBe(true);
      if (!result.ok) continue;

      for (const key of SHARED_THEME_KEYS) {
        expect(result.preset.light[key], `${id} light ${key}`).toBeTruthy();
        expect(result.preset.dark[key], `${id} dark ${key}`).toBeTruthy();
      }
      for (const key of REQUIRED_LIGHT_DARK_KEYS) {
        expect(result.preset.light[key], `${id} light ${key}`).toBeTruthy();
        expect(result.preset.dark[key], `${id} dark ${key}`).toBeTruthy();
      }
      // Explicitly guard the custom faint token and canvas tokens.
      expect(result.preset.light['muted-foreground-faint']).toBeTruthy();
      expect(result.preset.dark['muted-foreground-faint']).toBeTruthy();
      for (const key of LIGHT_DARK_KEYS.filter((k) => k.startsWith('canvas-'))) {
        expect(result.preset.light[key], `${id} light ${key}`).toBeTruthy();
        expect(result.preset.dark[key], `${id} dark ${key}`).toBeTruthy();
      }
    }
  });

  test('fixtures contain no overlay-specific or unknown tokens', () => {
    for (const [id, fixture] of FIXTURES) {
      for (const section of ['theme', 'light', 'dark'] as const) {
        for (const key of Object.keys(fixture.cssVars[section])) {
          expect(ALLOWED_THEME_KEYS, `${id} ${section} ${key}`).toContain(key);
        }
      }
    }
  });

  test('manifest entries expose labels/swatches but fixtures do not', () => {
    for (const entry of PRESET_MANIFEST) {
      expect(entry.label.length).toBeGreaterThan(0);
      expect(entry.swatches.length).toBeGreaterThan(0);
    }
    for (const [, fixture] of FIXTURES) {
      expect('label' in fixture).toBe(false);
      expect('swatches' in fixture).toBe(false);
    }
  });
});

describe('normalization', () => {
  test('trims strings and preserves authored colour format', () => {
    const fixture = clone(amberFixture) as Record<string, any>;
    fixture.cssVars.light.background = '  #ABCDEF  ';
    const preset = normalizePreset(fixture);
    expect(preset.light.background).toBe('#ABCDEF');
  });

  test('ignores unknown keys with a dev warning', () => {
    const fixture = clone(amberFixture) as Record<string, any>;
    fixture.cssVars.light['overlay-tint'] = '#ffffff';
    const warnings: string[] = [];
    const preset = normalizePreset(fixture, { warn: (m) => warnings.push(m) });
    expect('overlay-tint' in preset.light).toBe(false);
    expect(warnings.some((m) => m.includes('overlay-tint'))).toBe(true);
  });

  test('mode-specific value wins on a duplicate shared key and warns', () => {
    const fixture = clone(amberFixture) as Record<string, any>;
    fixture.cssVars.light.radius = '1rem';
    const warnings: string[] = [];
    const preset = normalizePreset(fixture, { warn: (m) => warnings.push(m) });
    expect(preset.light.radius).toBe('1rem');
    expect(preset.dark.radius).toBe('0.5rem');
    expect(warnings.some((m) => m.includes('Duplicate token "radius"'))).toBe(true);
  });

  test('rejects missing required keys, empty/non-string values and malformed colours', () => {
    const missing = clone(amberFixture) as Record<string, any>;
    delete missing.cssVars.dark['muted-foreground-faint'];
    expect(() => normalizePreset(missing)).toThrow(ThemePresetError);

    const empty = clone(amberFixture) as Record<string, any>;
    empty.cssVars.light.primary = '   ';
    expect(() => normalizePreset(empty)).toThrow(ThemePresetError);

    const nonString = clone(amberFixture) as Record<string, any>;
    nonString.cssVars.light.primary = 123;
    expect(() => normalizePreset(nonString)).toThrow(ThemePresetError);

    const malformed = clone(amberFixture) as Record<string, any>;
    malformed.cssVars.dark.background = 'not-a-colour';
    expect(() => normalizePreset(malformed)).toThrow(ThemePresetError);
  });

  test('rejects css blocks and wrong shape/type/name', () => {
    const withCss = clone(amberFixture) as Record<string, any>;
    withCss.css = ':root { --background: red; }';
    expect(() => normalizePreset(withCss)).toThrow(ThemePresetError);

    const withCssVarsCss = clone(amberFixture) as Record<string, any>;
    withCssVarsCss.cssVars.css = ':root {}';
    expect(() => normalizePreset(withCssVarsCss)).toThrow(ThemePresetError);

    const wrongType = clone(amberFixture) as Record<string, any>;
    wrongType.type = 'registry:ui';
    expect(() => normalizePreset(wrongType)).toThrow(ThemePresetError);

    const badName = clone(amberFixture) as Record<string, any>;
    badName.name = 'Not A Preset';
    expect(() => normalizePreset(badName)).toThrow(ThemePresetError);

    expect(() => normalizePreset(null)).toThrow(ThemePresetError);
    expect(() => normalizePreset([])).toThrow(ThemePresetError);
  });

  test('validatePreset returns a non-throwing error result', () => {
    const bad = clone(amberFixture) as Record<string, any>;
    delete bad.cssVars.light.foreground;
    const result = validatePreset(bad);
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toContain('foreground');
  });

  test('colour validator accepts authored hex/functional formats', () => {
    expect(isValidColorValue('#fff')).toBe(true);
    expect(isValidColorValue('#11223344')).toBe(true);
    expect(isValidColorValue('rgb(1 2 3 / 0.5)')).toBe(true);
    expect(isValidColorValue('rgba(1, 2, 3, 0.5)')).toBe(true);
    expect(isValidColorValue('hsl(210 50% 40%)')).toBe(true);
    expect(isValidColorValue('hsla(210, 50%, 40%, 0.5)')).toBe(true);
    expect(isValidColorValue('oklch(0.7 0.1 45)')).toBe(true);
    expect(isValidColorValue('oklch(70% 0.1 45 / 0.5)')).toBe(true);
    expect(isValidColorValue('oklab(0.7 0.1 0.2)')).toBe(true);
    expect(isValidColorValue('lab(50% 20 -30)')).toBe(true);
    expect(isValidColorValue('lch(50% 30 120)')).toBe(true);
    expect(isValidColorValue('white')).toBe(true);
  });

  test('colour validator rejects malformed components and non-colour keywords', () => {
    expect(isValidColorValue('rgb(not-a-color)')).toBe(false);
    expect(isValidColorValue('rgb(1 2 nope)')).toBe(false);
    expect(isValidColorValue('hsl(210 50% nope)')).toBe(false);
    expect(isValidColorValue('oklch(0.7 0.1 nope)')).toBe(false);
    expect(isValidColorValue('color(display-p3 1 0 0)')).toBe(false);
    expect(isValidColorValue('inherit')).toBe(false);
    expect(isValidColorValue('currentColor')).toBe(false);
    expect(isValidColorValue('transparent')).toBe(false);
    expect(isValidColorValue('')).toBe(false);
    expect(isValidColorValue('nope')).toBe(false);
  });

  test('normalizePreset rejects non-colour keywords in semantic colour tokens', () => {
    for (const bad of ['transparent', 'currentColor', 'inherit', 'rgb(not-a-color)']) {
      const fixture = clone(amberFixture) as Record<string, any>;
      fixture.cssVars.dark.background = bad;
      expect(() => normalizePreset(fixture), bad).toThrow(ThemePresetError);
    }
  });
});

describe('preset id fallback', () => {
  test('known ids resolve to themselves', () => {
    for (const id of PRESET_IDS) {
      expect(isKnownPresetId(id)).toBe(true);
      expect(resolvePresetId(id)).toBe(id);
      expect(getPreset(id).id).toBe(id);
    }
  });

  test('invalid ids fall back to zoneflow-amber', () => {
    for (const bad of ['', 'unknown', 'ZONEFLOW-AMBER', null, undefined, 42, {}]) {
      expect(isKnownPresetId(bad)).toBe(false);
      expect(resolvePresetId(bad)).toBe(DEFAULT_PRESET_ID);
      expect(getPreset(bad).id).toBe(DEFAULT_PRESET_ID);
    }
  });
});
