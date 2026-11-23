/**
 * Appearance preference helpers for the ModeWatcher-backed mode/preset storage.
 *
 * These helpers are pure (no DOM/localStorage access) so they can be unit
 * tested. The pre-paint bootstrap in `index.html` mirrors the same validation
 * rules with a tiny inline allowlist of preset IDs; it must never duplicate
 * preset token values — generated CSS is the only source of token values.
 */

import {
  DEFAULT_PRESET_ID,
  isKnownPresetId,
  resolvePresetId,
} from '../../themes/manifest';

/** localStorage key used by ModeWatcher for the user's mode preference. */
export const MODE_STORAGE_KEY = 'mode-watcher-mode';
/** localStorage key used by ModeWatcher for the active preset id. */
export const THEME_STORAGE_KEY = 'mode-watcher-theme';

export type AppearanceMode = 'light' | 'dark' | 'system';

export const DEFAULT_MODE: AppearanceMode = 'system';

const MODES: readonly AppearanceMode[] = ['light', 'dark', 'system'];

/** Type guard for a stored mode value. */
export function isValidMode(value: unknown): value is AppearanceMode {
  return typeof value === 'string' && (MODES as readonly string[]).includes(value);
}

/** Invalid or missing modes resolve to `system`. */
export function parseMode(value: unknown): AppearanceMode {
  return isValidMode(value) ? value : DEFAULT_MODE;
}

/** Type guard for a stored preset id (delegates to the manifest allowlist). */
export function isValidPresetId(value: unknown): boolean {
  return isKnownPresetId(value);
}

/** Invalid or unknown preset ids resolve to `zoneflow-amber`. */
export function parsePresetId(value: unknown): string {
  return resolvePresetId(value);
}

export interface AppearancePreference {
  mode: AppearanceMode;
  presetId: string;
}

export const DEFAULT_APPEARANCE: AppearancePreference = {
  mode: DEFAULT_MODE,
  presetId: DEFAULT_PRESET_ID,
};

/** Normalize a pair of raw stored values into a valid appearance preference. */
export function normalizeAppearance(mode: unknown, presetId: unknown): AppearancePreference {
  return { mode: parseMode(mode), presetId: parsePresetId(presetId) };
}

/** Resolve a mode to a concrete `light`/`dark` value using the OS preference. */
export function resolveMode(mode: AppearanceMode, systemPrefersDark: boolean): 'light' | 'dark' {
  if (mode === 'system') return systemPrefersDark ? 'dark' : 'light';
  return mode;
}
