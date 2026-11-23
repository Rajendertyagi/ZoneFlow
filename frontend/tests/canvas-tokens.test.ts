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
