/// <reference types="node" />
// @ts-ignore - bun:test types are provided by the Bun runtime, not @types/node.
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { GENERATED_CSS_PATH, generateCss } from './generate-css';
import { PRESET_MANIFEST } from './manifest';
import {
  OPTIONAL_LIGHT_DARK_KEYS,
  REQUIRED_LIGHT_DARK_KEYS,
  SHARED_THEME_KEYS,
} from './schema';

const here = dirname(fileURLToPath(import.meta.url));
const APP_CSS_PATH = resolve(here, '..', 'app.css');

function block(css: string, selector: string): string {
  const marker = `${selector} {`;
  const start = css.indexOf(marker);
  if (start === -1) return '';
  const open = css.indexOf('{', start);
  const close = css.indexOf('}', open);
  return css.slice(open + 1, close);
}

describe('generated/presets.css', () => {
  test('committed output is current with the fixtures', () => {
    const committed = readFileSync(GENERATED_CSS_PATH, 'utf8');
    expect(committed).toBe(generateCss());
  });

  test('emits the exact required selector for every preset and both modes', () => {
    const css = generateCss();
    for (const item of PRESET_MANIFEST) {
      expect(css).toContain(`:root[data-theme="${item.id}"] {`);
      expect(css).toContain(`:root[data-theme="${item.id}"].dark {`);
    }
  });

  test('every block contains every required token with the fixture value', () => {
    const css = generateCss();
    for (const item of PRESET_MANIFEST) {
      for (const [mode, selector] of [
        ['light', `:root[data-theme="${item.id}"]`],
        ['dark', `:root[data-theme="${item.id}"].dark`],
      ] as const) {
        const tokens = item.preset[mode];
        const body = block(css, selector);
        expect(body, `${item.id} ${mode} block missing`).not.toBe('');
        for (const key of [...SHARED_THEME_KEYS, ...REQUIRED_LIGHT_DARK_KEYS]) {
          expect(body, `${item.id} ${mode} missing --${key}`).toContain(`--${key}:`);
          expect(body, `${item.id} ${mode} wrong value for --${key}`).toContain(
            `--${key}: ${tokens[key]};`,
          );
        }
        // Optional keys are emitted with exact values whenever a fixture has
        // them (chart-1..chart-5 in the current built-ins).
        for (const key of OPTIONAL_LIGHT_DARK_KEYS) {
          const value = tokens[key];
          if (value === undefined) continue;
          expect(body, `${item.id} ${mode} missing optional --${key}`).toContain(`--${key}:`);
          expect(body, `${item.id} ${mode} wrong optional value for --${key}`).toContain(
            `--${key}: ${value};`,
          );
        }
      }
    }
  });

  test('current fixtures provide all optional chart keys in both modes', () => {
    for (const item of PRESET_MANIFEST) {
      for (const mode of ['light', 'dark'] as const) {
        for (const key of OPTIONAL_LIGHT_DARK_KEYS) {
          expect(item.preset[mode][key], `${item.id} ${mode} ${key}`).toBeTruthy();
        }
      }
    }
  });

  test('shared keys are emitted under distinct internal names for Tailwind wiring', () => {
    const css = generateCss();
    for (const item of PRESET_MANIFEST) {
      for (const [mode, selector] of [
        ['light', `:root[data-theme="${item.id}"]`],
        ['dark', `:root[data-theme="${item.id}"].dark`],
      ] as const) {
        const body = block(css, selector);
        const tokens = item.preset[mode];
        expect(body, `${item.id} ${mode} --zf-radius`).toContain(
          `--zf-radius: ${tokens.radius};`,
        );
        expect(body, `${item.id} ${mode} --zf-font-sans`).toContain(
          `--zf-font-sans: ${tokens['font-sans']};`,
        );
        expect(body, `${item.id} ${mode} --zf-font-mono`).toContain(
          `--zf-font-mono: ${tokens['font-mono']};`,
        );
      }
    }
  });

  test('default preset blocks are present so the app renders before WP2', () => {
    const css = generateCss();
    expect(block(css, ':root')).toContain('--background:');
    expect(block(css, ':root.dark')).toContain('--background:');
  });

  test('no overlay-specific tokens leak into generated CSS', () => {
    const css = generateCss();
    expect(css).not.toContain('--overlay-');
    expect(css).not.toContain('--zone-tint');
  });
});

describe('app.css wiring', () => {
  const css = readFileSync(APP_CSS_PATH, 'utf8');

  test('imports generated CSS and declares semantic @theme inline mappings', () => {
    expect(css).toContain("@import './themes/generated/presets.css';");
    expect(css).toContain('@theme inline');
    expect(css).toContain('--color-background: var(--background);');
    expect(css).toContain('--color-muted-foreground-faint: var(--muted-foreground-faint);');
    expect(css).toContain('--color-canvas-preview: var(--canvas-preview);');
  });

  test('radius/font-sans/font-mono map to preset variables without self-reference', () => {
    expect(css).toContain('--radius: var(--zf-radius);');
    expect(css).toContain('--font-sans: var(--zf-font-sans);');
    expect(css).toContain('--font-mono: var(--zf-font-mono);');
    // No hardcoded font literal may override the preset-driven values.
    expect(css).not.toContain("'Segoe UI'");
    expect(css).not.toContain("'Cascadia Code'");
    expect(css).not.toContain('--font-sans: var(--font-sans)');
    expect(css).not.toContain('--font-mono: var(--font-mono)');
    // Body styling resolves through the preset-driven variable.
    expect(css).toContain('font-family: var(--font-sans);');
  });

  test('zf-faint maps exactly to muted-foreground-faint', () => {
    expect(css).toContain('--color-zf-faint: var(--muted-foreground-faint);');
  });

  test('all legacy zf aliases are preserved and point at semantic tokens', () => {
    const aliases: Record<string, string> = {
      'zf-bg': 'background',
      'zf-surface': 'card',
      'zf-panel': 'popover',
      'zf-raised': 'secondary',
      'zf-border': 'border',
      'zf-border-strong': 'input',
      'zf-text': 'foreground',
      'zf-muted': 'muted-foreground',
      'zf-faint': 'muted-foreground-faint',
      'zf-accent': 'primary',
      'zf-accent-hover': 'primary-hover',
      'zf-accent-ink': 'primary-foreground',
      'zf-ok': 'success',
      'zf-warn': 'warning',
      'zf-error': 'destructive',
    };
    for (const [alias, semantic] of Object.entries(aliases)) {
      expect(css, `--color-${alias}`).toContain(`--color-${alias}: var(--${semantic});`);
    }
  });
});
