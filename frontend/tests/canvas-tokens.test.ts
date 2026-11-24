/**
 * WP4 — static guard for the theme-aware canvas token migration.
 *
 * No DOM harness and no new dependencies: this reads the Canvas sources as
 * text and asserts (a) no hardcoded hex colours remain outside the single
 * documented exception, (b) the expected semantic var() references are present
 * with distinct states preserved, and (c) the generated CSS exposes every
 * `--canvas-*` token in EVERY preset/mode selector block (`:root`, `:root.dark`
 * and each `:root[data-theme="..."]` / `.dark` variant) so SVG
 * `fill="var(--canvas-*)"` resolves in every built-in preset and mode.
 */
/// <reference types="node" />
// @ts-ignore - bun:test types are provided by the Bun runtime, not @types/node.
import { describe, expect, test } from 'bun:test';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PRESET_IDS } from '../src/themes/manifest';

const here = dirname(fileURLToPath(import.meta.url));
const canvasDir = resolve(here, '..', 'src', 'lib', 'components', 'Canvas');
const generatedCssPath = resolve(here, '..', 'src', 'themes', 'generated', 'presets.css');

const HEX = /#[0-9a-fA-F]{3,8}\b/g;

function read(name: string): string {
  return readFileSync(resolve(canvasDir, name), 'utf8');
}

/**
 * Documented exception: `zone_tints` are user-configurable overlay/zone tint
 * defaults from workspace settings, independent of the editor theme (Locked
 * Decision 9 and the plan's hardcoded-colour inventory). They are not theme
 * colours and have no sensible existing semantic token, so they stay literal.
 */
const ZONE_TINTS_LINE =
  "    zone_tints: ['#F5B544', '#6EA8FE', '#5BD6A0', '#C792EA', '#FF8A65', '#4DD0E1'],";
const ZONE_TINT_HEX = ['#F5B544', '#6EA8FE', '#5BD6A0', '#C792EA', '#FF8A65', '#4DD0E1'];

function withoutExceptions(source: string): string {
  return source.split(ZONE_TINTS_LINE).join('');
}

/** The complete canvas-specific token set required by the plan (WP1 schema). */
const CANVAS_TOKENS = [
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
] as const;

/** Parse every top-level `selector { ... }` block in the generated CSS. */
function parseBlocks(css: string): Map<string, string> {
  const withoutComments = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const blocks = new Map<string, string>();
  const pattern = /([^{}]+)\{([^{}]*)\}/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(withoutComments)) !== null) {
    blocks.set(match[1].trim(), match[2]);
  }
  return blocks;
}

describe('WP4 canvas token migration', () => {
  const files = readdirSync(canvasDir).filter((file) => file.endsWith('.svelte'));

  test('Canvas sources contain no hardcoded hex colours outside documented exceptions', () => {
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) {
      const cleaned = withoutExceptions(read(file));
      const matches = cleaned.match(HEX) ?? [];
      expect(matches, `${file} still has hardcoded hex: ${matches.join(', ')}`).toEqual([]);
    }
  });

  test('the only remaining hex colours are the documented zone_tints fallback', () => {
    const source = read('ZoneCanvas.svelte');
    expect(source).toContain(ZONE_TINTS_LINE);
    for (const hex of source.match(HEX) ?? []) {
      expect(ZONE_TINT_HEX, `unexpected hex ${hex}`).toContain(hex);
    }
  });

  test('ZoneCanvas uses canvas-grid / label / muted-faint / primary tokens', () => {
    const source = read('ZoneCanvas.svelte');
    expect(source).toContain('stroke="var(--canvas-grid)"');
    expect(source).toContain('fill="var(--canvas-label-fg)"');
    expect(source).toContain('fill="var(--muted-foreground-faint)"');
    expect(source).toContain("?? 'var(--primary)'");
  });

  test('ZoneRect preserves distinct selected/drop/merge/stroke/handle/label states', () => {
    const source = read('ZoneRect.svelte');
    // Four distinct zone-border states — none collapsed into one colour.
    expect(source).toContain("'var(--canvas-zone-drop)'");
    expect(source).toContain("'var(--canvas-zone-merge)'");
    expect(source).toContain("'var(--canvas-zone-selected)'");
    expect(source).toContain("'var(--canvas-zone-stroke)'");
    // Badge background, primary label, secondary label, on-accent text.
    expect(source).toContain('fill="var(--canvas-label-bg)"');
    expect(source).toContain('fill="var(--canvas-label-fg)"');
    expect(source).toContain('fill="var(--muted-foreground)"');
    expect(source).toContain('fill="var(--primary-foreground)"');
    // Shared vs outer handles stay visually distinct.
    expect(source).toContain("'var(--canvas-handle-outer)'");
    expect(source).toContain("'var(--canvas-handle-shared)'");
  });

  test('DragPreview uses canvas-preview for fill/stroke and canvas-label-fg for text', () => {
    const source = read('DragPreview.svelte');
    expect(source).toContain('stroke="var(--canvas-preview)"');
    expect(source).toContain('fill="var(--canvas-preview)"');
    expect(source).toContain('fill="var(--canvas-label-fg)"');
    expect(source).toContain("?? 'var(--canvas-preview)'");
  });

  test('SnapGuides uses the exact locked mapping', () => {
    const source = read('SnapGuides.svelte');
    // guide strokes -> canvas-preview; HUD bg -> canvas-label-bg;
    // HUD border -> canvas-preview; HUD text -> canvas-label-fg.
    expect(source).toContain('stroke="var(--canvas-preview)"');
    expect(source).toContain('fill="var(--canvas-label-bg)"');
    expect(source).toContain('fill="var(--canvas-label-fg)"');
  });

  test('generated CSS exposes every canvas token in every preset/mode selector block', () => {
    const css = readFileSync(generatedCssPath, 'utf8');
    const blocks = parseBlocks(css);

    // Default light/dark blocks plus both modes of every built-in preset.
    const expectedSelectors = [
      ':root',
      ':root.dark',
      ...PRESET_IDS.flatMap((id) => [
        `:root[data-theme="${id}"]`,
        `:root[data-theme="${id}"].dark`,
      ]),
    ];

    // No block may be silently skipped: the parsed set must match exactly.
    expect([...blocks.keys()].sort()).toEqual([...expectedSelectors].sort());

    for (const selector of expectedSelectors) {
      const body = blocks.get(selector);
      expect(body, `missing CSS block for ${selector}`).toBeDefined();
      for (const token of CANVAS_TOKENS) {
        expect(body, `--${token} missing in ${selector}`).toContain(`--${token}:`);
      }
    }
  });
});

/**
 * VWP4 — visual refinement guards. These lock in the token-only treatment so a
 * later edit cannot silently reintroduce a hardcoded colour or a new token.
 * Geometry, snapping and pointer logic are deliberately NOT asserted here.
 */
describe('VWP4 canvas visual refinement', () => {
  const files = readdirSync(canvasDir).filter((file) => file.endsWith('.svelte'));

  test('ZoneCanvas uses the canvas-bg token and a subtler resting grid', () => {
    const source = read('ZoneCanvas.svelte');
    expect(source).toContain('fill="var(--canvas-bg)"');
    expect(source).toContain("mode === 'draw' ? 0.5 : 0.14");
  });

  test('ZoneCanvas empty state is a tokenized placeholder card', () => {
    const source = read('ZoneCanvas.svelte');
    expect(source).toContain('No zones yet');
    expect(source).toContain('fill="var(--canvas-label-bg)"');
    expect(source).toContain('stroke="var(--canvas-zone-stroke)"');
    expect(source).toContain('stroke="var(--canvas-preview)"');
  });

  test('ZoneRect keeps four distinct fill opacities and a label halo', () => {
    const source = read('ZoneRect.svelte');
    expect(source).toContain(
      'fill-opacity={dropTarget ? 0.24 : selected ? 0.2 : mergeSource ? 0.16 : 0.08}',
    );
    expect(source).toContain('paint-order="stroke"');
    expect(source).toContain('stroke="var(--canvas-label-bg)"');
    // Badge number and the primary label both use the legible label foreground.
    expect(source.match(/fill="var\(--canvas-label-fg\)"/g)?.length ?? 0).toBeGreaterThanOrEqual(2);
  });

  test('ZoneRect handles are larger and higher-contrast', () => {
    const source = read('ZoneRect.svelte');
    expect(source).toContain('Math.max(12, workArea.w * 0.0075)');
    expect(source).toContain('stroke-opacity={0.85}');
    expect(source).toContain('stroke-width={1.5}');
  });

  test('SnapGuides resting guides are subtler than the snapped state', () => {
    const source = read('SnapGuides.svelte');
    expect(source).toContain('emphasized ? 0.22 : 0.08');
    expect(source).toContain('? 2.5 : 1');
  });

  test('DragPreview readouts sit on a tokenized backdrop', () => {
    const source = read('DragPreview.svelte');
    expect(source).toContain('fill="var(--canvas-label-bg)"');
    expect(source).toContain('labelWidth(coordText)');
    expect(source).toContain('labelWidth(drawText)');
    expect(source).toContain('fill-opacity={0.2}');
    expect(source).toContain('fill-opacity={0.18}');
  });

  test('canvas sources reference only the existing token vocabulary (no new tokens)', () => {
    // canvas-* tokens plus the semantic tokens already used by the canvas.
    const allowed = new Set<string>([
      ...CANVAS_TOKENS.map((token) => `--${token}`),
      '--muted-foreground',
      '--muted-foreground-faint',
      '--primary',
      '--primary-foreground',
      '--font-mono',
    ]);
    const varPattern = /var\((--[a-z0-9-]+)\)/g;
    for (const file of files) {
      const source = read(file);
      const used = new Set<string>();
      let match: RegExpExecArray | null;
      while ((match = varPattern.exec(source)) !== null) used.add(match[1]);
      for (const token of used) {
        expect(allowed.has(token), `${file} references unexpected token ${token}`).toBe(true);
      }
    }
  });
});
