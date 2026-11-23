/**
 * Built-in preset manifest. The JSON fixtures are the source of truth for
 * token values; `label` and `swatches` are ZoneFlow-only presentation metadata
 * and intentionally live here rather than in the tweakcn-compatible fixtures.
 */
import amberFixture from './presets/zoneflow-amber.json';
import caffeineFixture from './presets/caffeine.json';
import { normalizePreset, type NormalizedThemePreset } from './schema';

export const DEFAULT_PRESET_ID = 'zoneflow-amber';

export interface PresetManifestEntry {
  /** Stable id used as the `data-theme` value and localStorage preset value. */
  id: string;
  /** Human-readable label for appearance controls (manifest metadata only). */
  label: string;
  /** Small colour chips for the preset picker. */
  swatches: readonly string[];
  /** Validated, normalized token maps for both modes. */
  preset: NormalizedThemePreset;
}

function entry(
  id: string,
  label: string,
  swatches: readonly string[],
  fixture: unknown,
): PresetManifestEntry {
  const preset = normalizePreset(fixture);
  if (preset.name !== id) {
    throw new Error(`Preset fixture name "${preset.name}" does not match manifest id "${id}".`);
  }
  return { id, label, swatches, preset };
}

export const PRESET_MANIFEST: readonly PresetManifestEntry[] = Object.freeze([
  entry('zoneflow-amber', 'ZoneFlow Amber', ['#f59e0b', '#070b14', '#93a4bd'], amberFixture),
  entry('caffeine', 'Caffeine', ['#d6a87c', '#181512', '#a8a29e'], caffeineFixture),
]);

export const PRESET_IDS: readonly string[] = Object.freeze(PRESET_MANIFEST.map((item) => item.id));

/** Type guard for stored preset ids. */
export function isKnownPresetId(id: unknown): id is string {
  return typeof id === 'string' && PRESET_IDS.includes(id);
}

/** Invalid or unknown ids always resolve to the default preset. */
export function resolvePresetId(id: unknown): string {
  return isKnownPresetId(id) ? id : DEFAULT_PRESET_ID;
}

/** Manifest lookup with default fallback; never returns undefined. */
export function getPreset(id: unknown): PresetManifestEntry {
  const resolved = resolvePresetId(id);
  const found = PRESET_MANIFEST.find((item) => item.id === resolved);
  // `resolvePresetId` guarantees a known id, so this is unreachable in practice.
  return found ?? PRESET_MANIFEST[0];
}
