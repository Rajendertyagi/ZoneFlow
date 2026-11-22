/**
 * Regression guard for the latent D3 bypass in ZoneCanvas.svelte.
 *
 * Assignment must always flow through the parent-owned `onassign` callback
 * (which calls `assignAppGuarded`). A direct `api.assignZone` fallback would
 * silently skip the confirmation + rule-reset guard, so the prop is required
 * and no such fallback may exist. Bun cannot render Svelte components, so this
 * pins the invariant at the source level.
 */
import { describe, expect, test } from 'bun:test';

const source = await Bun.file(
  new URL('../src/lib/components/Canvas/ZoneCanvas.svelte', import.meta.url),
).text();

describe('ZoneCanvas assignment is always guarded (D3)', () => {
  test('onassign is a required prop (no optional marker)', () => {
    expect(source).toMatch(/onassign:\s*\(zoneId:\s*number,\s*spec:\s*AssignDto\)/);
    expect(source).not.toMatch(/onassign\s*\?\s*:/);
  });

  test('no direct api.assignZone call remains in ZoneCanvas', () => {
    // Comments may mention it as a warning; only an actual call is a failure.
    const code = source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/[^\n]*/g, '');
    expect(code).not.toMatch(/api\.assignZone\s*\(/);
  });

  test('the drop handler delegates to the guarded onassign callback', () => {
    expect(source).toMatch(/void\s+onassign\s*\(\s*zone\.id\s*,\s*spec\s*\)/);
  });
});
