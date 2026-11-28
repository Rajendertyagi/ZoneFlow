/**
 * VWP1 / WP-O4 — static guard for dialog & scrim tokenization.
 *
 * No DOM harness and no new dependencies: this reads the dialog sources as
 * text and asserts (a) no hardcoded hex colours remain outside the single
 * documented overlay exception, (b) the generated `dialog-overlay.svelte`
 * uses the canonical `bg-scrim` utility (never `bg-black/*`), (c) `dialogs/*`
 * contain no hand-rolled scrim button or `role="dialog"` markup, (d) `app.css`
 * maps `--color-scrim` in `@theme inline`, and (e) the generated CSS exposes
 * `--scrim` in EVERY preset/mode selector block so `bg-scrim` resolves in
 * every built-in preset.
 */
/// <reference types="node" />
// @ts-ignore - bun:test types are provided by the Bun runtime, not @types/node.
import { describe, expect, test } from 'bun:test';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PRESET_IDS } from '../src/themes/manifest';

const here = dirname(fileURLToPath(import.meta.url));
const dialogsDir = resolve(here, '..', 'src', 'lib', 'components', 'dialogs');
const dialogOverlayPath = resolve(
  here,
  '..',
  'src',
  'lib',
  'components',
  'ui',
  'dialog',
  'dialog-overlay.svelte',
);
const appCssPath = resolve(here, '..', 'src', 'app.css');
const generatedCssPath = resolve(here, '..', 'src', 'themes', 'generated', 'presets.css');

const HEX = /#[0-9a-fA-F]{3,8}\b/g;

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

/**
 * Documented exception: `zone_tints` are user-configurable overlay/zone tint
 * defaults from workspace settings, independent of the editor theme (Locked
 * Decision 5 and the plan's hardcoded-colour inventory). They are not theme
 * colours and have no sensible existing semantic token, so they stay literal.
 */
const ZONE_TINTS_LINE =
  "  const tints = $derived(store.snapshot?.settings.zone_tints ?? ['#F5B544', '#6EA8FE', '#5BD6A0', '#C792EA']);";
const ZONE_TINT_HEX = ['#F5B544', '#6EA8FE', '#5BD6A0', '#C792EA'];

function withoutDocumentedExceptions(source: string): string {
  return source.split(ZONE_TINTS_LINE).join('');
}

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

describe('VWP1 dialog & scrim tokenization', () => {
  const files = readdirSync(dialogsDir).filter((file) => file.endsWith('.svelte'));

  test('dialog sources contain no hardcoded hex outside the documented tints fallback', () => {
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) {
      const cleaned = withoutDocumentedExceptions(read(resolve(dialogsDir, file)));
      const matches = cleaned.match(HEX) ?? [];
      expect(matches, `${file} still has hardcoded hex: ${matches.join(', ')}`).toEqual([]);
    }
  });

  test('the only remaining dialog hex is the documented zone_tints fallback', () => {
    const source = read(resolve(dialogsDir, 'WizardDialog.svelte'));
    expect(source).toContain(ZONE_TINTS_LINE);
    for (const hex of source.match(HEX) ?? []) {
      expect(ZONE_TINT_HEX, `unexpected hex ${hex}`).toContain(hex);
    }
  });

  test('WizardDialog uses semantic tokens for preview fill/stroke and label text', () => {
    const source = read(resolve(dialogsDir, 'WizardDialog.svelte'));
    expect(source).toContain("?? 'var(--canvas-preview)'");
    expect(source).toContain('fill="var(--canvas-label-fg)"');
    expect(source).not.toContain('#f59e0b');
    expect(source).not.toContain('#e6edf7');
  });

  test('no bg-black/* or bg-[var(--scrim)] remains in dialogs', () => {
    for (const file of files) {
      const source = read(resolve(dialogsDir, file));
      expect(source, `${file} must not use bg-black/*`).not.toMatch(/bg-black\//);
      expect(source, `${file} must not use bg-[var(--scrim)]`).not.toContain('bg-[var(--scrim)]');
    }
  });

  test('generated dialog-overlay.svelte uses bg-scrim with no bg-black default', () => {
    const overlay = read(dialogOverlayPath);
    expect(overlay).toContain('bg-scrim');
    expect(overlay).not.toMatch(/bg-black\//);
  });

  test('dialogs/* contain no hand-rolled scrim button or role="dialog" markup', () => {
    for (const file of files) {
      const source = read(resolve(dialogsDir, file));
      expect(source, `${file} must not declare role="dialog"`).not.toContain('role="dialog"');
      expect(source, `${file} must not use bg-scrim`).not.toContain('bg-scrim');
      expect(source, `${file} must not use bg-black/*`).not.toMatch(/bg-black\//);
      expect(
        source,
        `${file} must not contain a hand-rolled scrim button`,
      ).not.toMatch(/<button\b[^>]*\bbg-(?:scrim|black\/)/);
    }
  });

  test('app.css @theme inline maps --color-scrim to var(--scrim)', () => {
    const css = read(appCssPath);
    expect(css).toContain('@theme inline');
    expect(css).toContain('--color-scrim: var(--scrim);');
    // No zf-* alias for scrim (bg-scrim is the canonical utility form).
    expect(css).not.toContain('--color-zf-scrim');
  });

  test('generated CSS requires scrim in every preset/mode block', () => {
    const css = read(generatedCssPath);
    const blocks = parseBlocks(css);
    const expectedSelectors = [
      ':root',
      ':root.dark',
      ...PRESET_IDS.flatMap((id) => [
        `:root[data-theme="${id}"]`,
        `:root[data-theme="${id}"].dark`,
      ]),
    ];
    expect([...blocks.keys()].sort()).toEqual([...expectedSelectors].sort());
    for (const selector of expectedSelectors) {
      const body = blocks.get(selector);
      expect(body, `missing CSS block for ${selector}`).toBeDefined();
      expect(body, `--scrim missing in ${selector}`).toContain('--scrim:');
    }
  });
});
