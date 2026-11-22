import type { ZoneDto } from '../api/client';

/**
 * U-09: applying a template with `replace=true` reuses the first zone and
 * deletes every other zone. Count how many of those deleted zones carry an
 * application assignment (a non-empty exe), so the UI can warn before silently
 * dropping them. Pure so it can be unit-tested without Svelte or the API.
 */
export function countAssignedZonesToRemove(zones: readonly ZoneDto[], keepId: number): number {
  return zones.filter((zone) => zone.id !== keepId && zone.exe.trim() !== '').length;
}
