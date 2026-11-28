/**
 * VWP2 — static guard for the low-chrome shell & navigation restyle.
 *
 * No DOM harness and no new dependencies: this reads the shell sources as text
 * and asserts that (a) elevation tokens are defined in app.css ONLY (never in
 * preset JSON or generated CSS), (b) the shadow base is dark in both modes and
 * is not derived from --foreground, (c) the VWP1 scrim mapping is untouched,
 * (d) shell components contain no hardcoded shadow hex or built-in heavy
 * shadow utilities, (e) focus rings / disabled states / reduced motion and
 * meaningful separators survive, (f) top bar shows only brand + nav + close
 * (no status chips), and (g) the toast dismiss button keeps a >= 24x24 target.
 */
/// <reference types="node" />
// @ts-ignore - bun:test types are provided by the Bun runtime, not @types/node.
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const src = resolve(here, '..', 'src');
const appCssPath = resolve(src, 'app.css');
const appPath = resolve(src, 'App.svelte');
const topBarPath = resolve(src, 'lib', 'components', 'TopBar.svelte');
const statusBarPath = resolve(src, 'lib', 'components', 'StatusBar.svelte');
// WP-O2: the hand-rolled button/badge/input primitives were deleted; their
// guards now read the generated official sources plus the migrated call sites.
const buttonPath = resolve(src, 'lib', 'components', 'ui', 'button', 'button.svelte');
const badgePath = resolve(src, 'lib', 'components', 'ui', 'badge', 'badge.svelte');
const inputPath = resolve(src, 'lib', 'components', 'ui', 'input', 'input.svelte');
const presetPaths = [
  resolve(src, 'themes', 'presets', 'zoneflow-amber.json'),
  resolve(src, 'themes', 'presets', 'caffeine.json'),
];
const generatedCssPath = resolve(src, 'themes', 'generated', 'presets.css');

const HEX = /#[0-9a-fA-F]{3,8}\b/g;

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

const shellFiles = [
  ['App.svelte', appPath],
  ['TopBar.svelte', topBarPath],
  ['StatusBar.svelte', statusBarPath],
  ['ui/button/button.svelte', buttonPath],
  ['ui/badge/badge.svelte', badgePath],
  ['ui/input/input.svelte', inputPath],
] as const;

describe('VWP2 shell & navigation low-chrome tokens', () => {
  test('app.css defines the elevation tokens with a dark base in both modes', () => {
    const css = read(appCssPath);
    expect(css).toContain('--zf-shadow-color: rgb(15 23 42 / 0.16);');
    expect(css).toContain('--zf-shadow-color: rgb(0 0 0 / 0.55);');
    expect(css).toContain('--zf-shadow-1: 0 1px 2px var(--zf-shadow-color);');
    expect(css).toContain('--zf-shadow-2: 0 4px 14px var(--zf-shadow-color);');
    expect(css).toContain('--zf-shadow-3: 0 18px 48px var(--zf-shadow-color);');
    expect(css).toContain('--zf-hairline: color-mix(in oklab, var(--foreground)');
    // V4 directional separators (light + dark + @theme utility mappings).
    expect(css).toContain('--zf-shadow-topbar: 0 1px 0 rgba(255,255,255,.04), 0 4px 12px rgba(15,23,42,.16);');
    expect(css).toContain('--zf-shadow-rail: inset -1px 0 0 rgba(255,255,255,.05), 4px 0 14px rgba(15,23,42,.16);');
    expect(css).toContain('--zf-shadow-inspector: inset 1px 0 0 rgba(255,255,255,.05), -4px 0 14px rgba(15,23,42,.16);');
    expect(css).toContain('--zf-shadow-status: 0 -1px 0 rgba(255,255,255,.04), 0 -4px 12px rgba(15,23,42,.16);');
    expect(css).toContain('--zf-shadow-inset-ring: inset 0 0 0 1px rgba(15,23,42,.12);');
    expect(css).toContain('--zf-shadow-topbar: 0 1px 0 rgba(255,255,255,.05), 0 4px 12px rgba(0,0,0,.55);');
    expect(css).toContain('--zf-shadow-rail: inset -1px 0 0 rgba(255,255,255,.05), 4px 0 14px rgba(0,0,0,.55);');
    expect(css).toContain('--zf-shadow-inspector: inset 1px 0 0 rgba(255,255,255,.05), -4px 0 14px rgba(0,0,0,.55);');
    expect(css).toContain('--zf-shadow-status: 0 -1px 0 rgba(255,255,255,.05), 0 -4px 12px rgba(0,0,0,.55);');
    expect(css).toContain('--zf-shadow-inset-ring: inset 0 0 0 1px rgba(0,0,0,.55);');
    for (const name of ['topbar', 'rail', 'inspector', 'status', 'inset-ring']) {
      expect(css, `missing @theme mapping for shadow-zf-${name}`).toContain(
        `--shadow-zf-${name}: var(--zf-shadow-${name});`,
      );
    }
  });

  test('shadow base is dark in both modes and never derived from --foreground', () => {
    const css = read(appCssPath);
    const shadowColorDeclarations = css.match(/--zf-shadow-color:[^;]+;/g) ?? [];
    expect(shadowColorDeclarations.length).toBe(2);
    for (const declaration of shadowColorDeclarations) {
      expect(declaration).not.toContain('var(--foreground)');
      expect(declaration).toMatch(/rgb\(/);
    }
    // The dark-mode override must be present and stronger (higher alpha).
    expect(css).toMatch(/:root\.dark\s*\{[^}]*--zf-shadow-color:\s*rgb\(0 0 0 \/ 0\.55\)/s);
  });

  test('elevation tokens are defined in app.css only, not in preset JSON or generated CSS', () => {
    for (const path of [...presetPaths, generatedCssPath]) {
      const source = read(path);
      expect(source, `${path} must not define elevation tokens`).not.toContain('--zf-shadow');
      expect(source, `${path} must not define --zf-hairline`).not.toContain('--zf-hairline');
    }
  });

  test('VWP1 scrim mapping is present and unchanged', () => {
    const css = read(appCssPath);
    expect(css).toContain('@theme inline');
    expect(css).toContain('--color-scrim: var(--scrim);');
    expect(css).not.toContain('--color-zf-scrim');
  });

  test('shell components contain no hardcoded hex colours', () => {
    for (const [name, path] of shellFiles) {
      const matches = read(path).match(HEX) ?? [];
      expect(matches, `${name} has hardcoded hex: ${matches.join(', ')}`).toEqual([]);
    }
  });

  test('shell components use only allowlisted elevation shadow utilities', () => {
    // Any Tailwind shadow utility is rejected unless it is one of the
    // app.css-defined elevation or V4 directional tokens. Variant prefixes
    // (hover:, md:, …) and the important marker (!) are ignored when checking
    // the base utility, and arbitrary values such as shadow-[...] are rejected
    // because they are not on the allowlist.
    const allowed = new Set([
      'shadow-zf-1', 'shadow-zf-2', 'shadow-zf-3',
      'shadow-zf-topbar', 'shadow-zf-rail', 'shadow-zf-inspector',
      'shadow-zf-status', 'shadow-zf-inset-ring',
    ]);
    const shadowUtility =
      /(?<![\w-])((?:[a-z][a-z0-9-]*:)*!?shadow(?:-(?:\[[^\]]+\]|[a-z0-9-]+))?)/g;

    for (const [name, path] of shellFiles) {
      const source = read(path);
      for (const match of source.matchAll(shadowUtility)) {
        const token = match[1];
        const base = token.replace(/^([a-z][a-z0-9-]*:)*!?/, '');
        expect(
          allowed.has(base),
          `${name} uses disallowed shadow utility "${token}" (base "${base}")`,
        ).toBe(true);
      }
    }
    expect(read(appPath)).toContain('shadow-zf-2'); // toasts
    expect(read(topBarPath)).toContain('shadow-zf-1'); // nav container
  });

  test('focus rings, disabled states and reduced motion are preserved', () => {
    // Generated official sources keep their own focus/disabled affordances.
    const button = read(buttonPath);
    expect(button).toContain('focus-visible:ring-3');
    expect(button).toContain('disabled:pointer-events-none disabled:opacity-50');

    const input = read(inputPath);
    expect(input).toContain('disabled:cursor-not-allowed disabled:opacity-50');

    // The repo's outline focus ring / reduced-motion convention is preserved at
    // the migrated Button call sites via class overrides.
    const buttonCallSites = [
      resolve(src, 'lib', 'views', 'EditorView.svelte'),
      resolve(src, 'lib', 'views', 'HomeView.svelte'),
      resolve(src, 'lib', 'views', 'SettingsView.svelte'),
      resolve(src, 'lib', 'components', 'dialogs', 'ConfirmDialog.svelte'),
    ];
    for (const path of buttonCallSites) {
      const source = read(path);
      expect(source, `${path} missing focus-ring override`).toContain('focus-visible:outline-2');
      expect(source, `${path} missing focus-ring override`).toContain('focus-visible:outline-primary');
      expect(source, `${path} missing reduced-motion override`).toContain('motion-reduce:transition-none');
    }

    // Input borders and the primary focus border are an accessibility
    // affordance; they are passed as class overrides at every Input call site.
    const inputCallSites = [
      resolve(src, 'lib', 'views', 'HotkeysView.svelte'),
      resolve(src, 'lib', 'views', 'SettingsView.svelte'),
      resolve(src, 'lib', 'components', 'dialogs', 'PickerDialog.svelte'),
      resolve(src, 'lib', 'components', 'dialogs', 'PromptDialog.svelte'),
      resolve(src, 'lib', 'components', 'Inspector', 'ZoneInspector.svelte'),
      resolve(src, 'lib', 'components', 'LeftRail', 'AppLibrary.svelte'),
    ];
    for (const path of inputCallSites) {
      const source = read(path);
      expect(source, `${path} missing input border`).toContain('border border-border');
      expect(source, `${path} missing input focus border`).toContain('focus-visible:border-primary');
    }
  });

  test('decorative shell borders are replaced by directional shadows', () => {
    const topBar = read(topBarPath);
    const statusBar = read(statusBarPath);
    expect(topBar).toContain('shadow-zf-topbar');
    expect(topBar).not.toContain('border-b border-zf-border');
    expect(statusBar).toContain('shadow-zf-status');
    expect(statusBar).not.toContain('border-t border-zf-border');
  });

  test('top bar shows only brand + nav + close (no status chips)', () => {
    const topBar = read(topBarPath);
    // P0b: status chips and the connection badge are gone on every view.
    for (const label of ['applied', 'selected', 'saved', 'layout']) {
      expect(topBar, `stale status chip label "${label}" still present`).not.toContain(`>${label}<`);
    }
    expect(topBar).not.toContain('rounded-full');
    expect(topBar).not.toContain('<Badge');
    expect(topBar).not.toContain('connectionClass');
    // Brand, nav and the close button survive unchanged.
    expect(topBar).toContain('ZoneFlow');
    expect(topBar).toContain('aria-label="Main navigation"');
    expect(topBar).toContain('aria-label="Close editor"');
  });

  test('toast dismiss button keeps at least a 24x24 target', () => {
    const app = read(appPath);
    expect(app).toContain('aria-label="Dismiss notification"');
    expect(app).toMatch(/aria-label="Dismiss notification"[\s\S]{0,160}h-6 w-6/);
  });

  test('toast and connection banner keep their ARIA roles', () => {
    const app = read(appPath);
    expect(app).toContain('role="alert"');
    expect(app).toContain('role="status"');
  });
});
