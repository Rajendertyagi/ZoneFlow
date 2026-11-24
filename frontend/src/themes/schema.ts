/**
 * Typed schema, runtime allowlists, normalization and validation for the
 * built-in ZoneFlow theme presets.
 *
 * The JSON fixtures in `./presets` are the single source of truth. This module
 * deliberately uses literal allowed-key types (never `Record<string, string>`)
 * so a typo in a fixture cannot silently become a theme token.
 */

export const SHARED_THEME_KEYS = ['radius', 'font-sans', 'font-mono'] as const;

export const REQUIRED_LIGHT_DARK_KEYS = [
  'background',
  'foreground',
  'card',
  'card-foreground',
  'popover',
  'popover-foreground',
  'primary',
  'primary-hover',
  'primary-foreground',
  'secondary',
  'secondary-foreground',
  'muted',
  'muted-foreground',
  'muted-foreground-faint',
  'accent',
  'accent-foreground',
  'destructive',
  'destructive-foreground',
  'success',
  'success-foreground',
  'warning',
  'warning-foreground',
  'border',
  'input',
  'ring',
  'canvas-bg',
  'canvas-grid',
  'canvas-zone-stroke',
  'canvas-zone-selected',
  'canvas-zone-drop',
  'canvas-zone-merge',
  'canvas-handle-shared',
  'canvas-handle-outer',
  'canvas-preview',
  'canvas-label-bg',
  'canvas-label-fg',
  'scrim',
] as const;

export const OPTIONAL_LIGHT_DARK_KEYS = [
  'chart-1',
  'chart-2',
  'chart-3',
  'chart-4',
  'chart-5',
] as const;

export const LIGHT_DARK_KEYS = [
  ...REQUIRED_LIGHT_DARK_KEYS,
  ...OPTIONAL_LIGHT_DARK_KEYS,
] as const;

export type SharedThemeKey = (typeof SHARED_THEME_KEYS)[number];
export type RequiredLightDarkKey = (typeof REQUIRED_LIGHT_DARK_KEYS)[number];
export type OptionalLightDarkKey = (typeof OPTIONAL_LIGHT_DARK_KEYS)[number];
export type LightDarkKey = (typeof LIGHT_DARK_KEYS)[number];
export type AllThemeKey = SharedThemeKey | LightDarkKey;

/** tweakcn-compatible registry-style fixture shape. */
export type ThemePresetFile = {
  $schema?: string;
  name: string;
  type: 'registry:style';
  cssVars: {
    theme: Record<SharedThemeKey, string>;
    light: Record<RequiredLightDarkKey, string> & Partial<Record<OptionalLightDarkKey, string>>;
    dark: Record<RequiredLightDarkKey, string> & Partial<Record<OptionalLightDarkKey, string>>;
  };
};

/** Flat, mode-specific token map used by the CSS generator. */
export type ThemeTokenMap = Record<SharedThemeKey | RequiredLightDarkKey, string> &
  Partial<Record<OptionalLightDarkKey, string>>;

export interface NormalizedThemePreset {
  name: string;
  type: 'registry:style';
  light: ThemeTokenMap;
  dark: ThemeTokenMap;
}

export class ThemePresetError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ThemePresetError';
  }
}

const SHARED_SET: ReadonlySet<string> = new Set<string>(SHARED_THEME_KEYS);
const LIGHT_DARK_SET: ReadonlySet<string> = new Set<string>(LIGHT_DARK_KEYS);
const REQUIRED_SET: ReadonlySet<string> = new Set<string>(REQUIRED_LIGHT_DARK_KEYS);
const ALLOWED_SET: ReadonlySet<string> = new Set<string>([
  ...SHARED_THEME_KEYS,
  ...LIGHT_DARK_KEYS,
]);

/** Exposed for tests and for the pre-paint allowlist in a later work package. */
export const ALLOWED_THEME_KEYS: readonly string[] = Object.freeze([...ALLOWED_SET]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

const HEX_COLOR = /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;

// Structural numeric checks only — no browser/CSS parser is available in the
// build environment. This intentionally rejects `color()`, `light-dark()`,
// relative-colour syntax and non-numeric components; those are not used by the
// built-in fixtures and are out of scope for this strict-enough validator.
const NUM = '[+-]?(?:\\d+(?:\\.\\d*)?|\\.\\d+)(?:[eE][+-]?\\d+)?';
const PCT = `${NUM}%?`;
const HUE = `${NUM}(?:deg|grad|rad|turn)?`;
const SEP = '\\s*[, ]\\s*';
const ALPHA = `(?:\\s*[,/]\\s*${PCT})?`;

const COLOR_FUNCTIONS: readonly RegExp[] = Object.freeze([
  new RegExp(`^rgba?\\(\\s*${PCT}${SEP}${PCT}${SEP}${PCT}${ALPHA}\\s*\\)$`, 'i'),
  new RegExp(`^hsla?\\(\\s*${HUE}${SEP}${PCT}${SEP}${PCT}${ALPHA}\\s*\\)$`, 'i'),
  new RegExp(`^hwb\\(\\s*${HUE}${SEP}${PCT}${SEP}${PCT}${ALPHA}\\s*\\)$`, 'i'),
  new RegExp(`^oklch\\(\\s*${PCT}${SEP}${NUM}${SEP}${HUE}${ALPHA}\\s*\\)$`, 'i'),
  new RegExp(`^oklab\\(\\s*${PCT}${SEP}${NUM}${SEP}${NUM}${ALPHA}\\s*\\)$`, 'i'),
  new RegExp(`^lab\\(\\s*${PCT}${SEP}${NUM}${SEP}${NUM}${ALPHA}\\s*\\)$`, 'i'),
  new RegExp(`^lch\\(\\s*${PCT}${SEP}${NUM}${SEP}${HUE}${ALPHA}\\s*\\)$`, 'i'),
]);

// `transparent`, `currentColor` and `inherit` are deliberately excluded: they
// are not valid values for a semantic preset colour role.
const NAMED_COLORS: ReadonlySet<string> = new Set([
  'white',
  'black',
  'red',
  'green',
  'blue',
  'gray',
  'grey',
  'orange',
  'yellow',
  'purple',
  'pink',
  'teal',
  'navy',
  'silver',
  'gold',
  'rebeccapurple',
]);

/** Reject obviously malformed color values while preserving authored format. */
export function isValidColorValue(value: string): boolean {
  const trimmed = value.trim();
  return (
    HEX_COLOR.test(trimmed) ||
    COLOR_FUNCTIONS.some((pattern) => pattern.test(trimmed)) ||
    NAMED_COLORS.has(trimmed.toLowerCase())
  );
}

export interface NormalizeOptions {
  /** Called for ignored keys / dedupe collisions. Defaults to a no-op. */
  warn?: (message: string) => void;
}

/** Internal accumulator; keys are still constrained to the literal allowlist. */
type MutableTokenMap = Partial<Record<AllThemeKey, string>>;

function collectSection(
  section: Record<string, unknown>,
  sectionName: 'theme' | 'light' | 'dark',
  targets: MutableTokenMap[],
  warn: (message: string) => void,
): void {
  for (const [key, rawValue] of Object.entries(section)) {
    if (!ALLOWED_SET.has(key)) {
      warn(`Ignoring unknown theme key "${key}" in cssVars.${sectionName}.`);
      continue;
    }
    if (typeof rawValue !== 'string' || rawValue.trim() === '') {
      throw new ThemePresetError(
        `Invalid value for "${key}" in cssVars.${sectionName}: expected a non-empty string.`,
      );
    }
    const value = rawValue.trim();
    if (LIGHT_DARK_SET.has(key) && !isValidColorValue(value)) {
      throw new ThemePresetError(
        `Invalid color value for "${key}" in cssVars.${sectionName}: "${rawValue}".`,
      );
    }

    const belongsToTheme = SHARED_SET.has(key);
    if (belongsToTheme && sectionName !== 'theme') {
      warn(`Shared theme key "${key}" found in cssVars.${sectionName}; mode-specific value wins.`);
    } else if (!belongsToTheme && sectionName === 'theme') {
      warn(`Mode token "${key}" found in cssVars.theme; mode-specific value wins.`);
    }

    const tokenKey = key as AllThemeKey;
    for (const target of targets) {
      if (Object.prototype.hasOwnProperty.call(target, tokenKey)) {
        warn(`Duplicate token "${key}" resolved with the mode-specific value.`);
      }
      target[tokenKey] = value;
    }
  }
}

/**
 * Validate and normalize an unknown fixture into flat light/dark token maps.
 * Throws {@link ThemePresetError} on any structural, key or value violation.
 */
export function normalizePreset(
  input: unknown,
  options: NormalizeOptions = {},
): NormalizedThemePreset {
  const warn = options.warn ?? (() => undefined);

  if (!isRecord(input)) {
    throw new ThemePresetError('Theme preset must be an object.');
  }
  if ('css' in input) {
    throw new ThemePresetError('Theme preset must not contain a `css` block.');
  }

  const name = input.name;
  if (typeof name !== 'string' || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name)) {
    throw new ThemePresetError('Theme preset "name" must be a non-empty kebab-case id.');
  }
  if (input.type !== 'registry:style') {
    throw new ThemePresetError('Theme preset "type" must be "registry:style".');
  }

  const cssVars = input.cssVars;
  if (!isRecord(cssVars)) {
    throw new ThemePresetError('Theme preset "cssVars" must be an object.');
  }
  if ('css' in cssVars) {
    throw new ThemePresetError('Theme preset cssVars must not contain a `css` block.');
  }

  const theme = cssVars.theme;
  const light = cssVars.light;
  const dark = cssVars.dark;
  if (!isRecord(theme) || !isRecord(light) || !isRecord(dark)) {
    throw new ThemePresetError('cssVars.theme, cssVars.light and cssVars.dark must be objects.');
  }

  const lightMap: MutableTokenMap = {};
  const darkMap: MutableTokenMap = {};

  // Shared tokens seed both modes; mode-specific blocks are applied afterwards
  // so a duplicate key is resolved in favour of the mode value.
  collectSection(theme, 'theme', [lightMap, darkMap], warn);
  collectSection(light, 'light', [lightMap], warn);
  collectSection(dark, 'dark', [darkMap], warn);

  for (const key of SHARED_THEME_KEYS) {
    if (!(key in lightMap) || !(key in darkMap)) {
      throw new ThemePresetError(`Missing required shared theme key "${key}".`);
    }
  }
  for (const key of REQUIRED_SET) {
    if (!(key in lightMap)) {
      throw new ThemePresetError(`Missing required light token "${key}".`);
    }
    if (!(key in darkMap)) {
      throw new ThemePresetError(`Missing required dark token "${key}".`);
    }
  }

  return {
    name,
    type: 'registry:style',
    light: lightMap as ThemeTokenMap,
    dark: darkMap as ThemeTokenMap,
  };
}

export type PresetValidationResult =
  | { ok: true; preset: NormalizedThemePreset }
  | { ok: false; error: string };

/** Non-throwing wrapper used by fallback lookup and tests. */
export function validatePreset(
  input: unknown,
  options?: NormalizeOptions,
): PresetValidationResult {
  try {
    return { ok: true, preset: normalizePreset(input, options) };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : String(error) };
  }
}
