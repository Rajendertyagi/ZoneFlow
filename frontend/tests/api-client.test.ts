/**
 * Regression test for the divider-resize undo fix.
 *
 * A divider drag must be committed with a single PUT /api/zones/batch request
 * carrying every affected zone, so the backend records one undo point for the
 * whole gesture instead of one per zone.
 */
import { afterEach, describe, expect, test } from 'bun:test';
import { api, type ZoneGeometryUpdateDto } from '../src/lib/api/client';

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe('updateZonesBatch', () => {
  test('sends one PUT with all geometry updates', async () => {
    const calls: Array<{ url: string; init: RequestInit }> = [];
    globalThis.fetch = (async (url: string, init: RequestInit) => {
      calls.push({ url, init });
      return new Response(JSON.stringify([{ id: 1 }, { id: 2 }]), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    }) as typeof fetch;

    const updates: ZoneGeometryUpdateDto[] = [
      { id: 1, x: 0, y: 0, w: 30, h: 100 },
      { id: 2, x: 30, y: 0, w: 70, h: 100 },
    ];
    const result = await api.updateZonesBatch(updates);

    expect(calls.length).toBe(1);
    expect(calls[0].url).toBe('/api/zones/batch');
    expect(calls[0].init.method).toBe('PUT');
    expect(JSON.parse(calls[0].init.body as string)).toEqual(updates);
    expect(result).toHaveLength(2);
  });
});
