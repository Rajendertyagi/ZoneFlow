/**
 * VWP3 — static guard for the low-chrome views & panels treatment.
 *
 * No DOM harness and no new dependencies: this reads the VWP3 sources as text
 * and asserts that (a) decorative `border border-zf-border` card/row wrappers
 * are gone from views and panels, (b) grouping is carried by surface contrast
 * plus the VWP2 elevation tokens (shadow-zf-1/2/3) and low-chrome hairlines,
 * (c) no hardcoded shadow hex or built-in heavy shadow utilities remain,
 * (d) meaningful separators, input borders, focus rings, keyboard helpers and
 * reduced motion survive, (e) pane widths/splits and 24x24 targets are kept,
 * and (f) AppearanceSettings stays isolated and outside the loaded-draft block.
 */
/// <reference types="node" />
// @ts-ignore - bun:test types are provided by the Bun runtime, not @types/node.
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const src = resolve(here, '..', 'src');
const views = resolve(src, 'lib', 'views');
const components = resolve(src, 'lib', 'components');
const ui = resolve(components, 'ui');

const paths = {
  home: resolve(views, 'HomeView.svelte'),
  editor: resolve(views, 'EditorView.svelte'),
  settings: resolve(views, 'SettingsView.svelte'),
  hotkeys: resolve(views, 'HotkeysView.svelte'),
  profiles: resolve(components, 'LeftRail', 'ProfilesPanel.svelte'),
  library: resolve(components, 'LeftRail', 'AppLibrary.svelte'),
  inspector: resolve(components, 'Inspector', 'ZoneInspector.svelte'),
  results: resolve(components, 'ApplyResults.svelte'),
  appearance: resolve(components, 'AppearanceSettings.svelte'),
  tabs: resolve(ui, 'tabs.svelte'),
  input: resolve(ui, 'input.svelte'),
  select: resolve(ui, 'select.svelte'),
  switch: resolve(ui, 'switch.svelte'),
  tooltip: resolve(ui, 'tooltip.svelte'),
  contextMenu: resolve(ui, 'context-menu.svelte'),
  resizable: resolve(ui, 'resizable.svelte'),
  scrollArea: resolve(ui, 'scroll-area.svelte'),
} as const;

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

const HEX = /#[0-9a-fA-F]{3,8}\b/g;

/** All VWP3 sources, including the UI primitives that were selectively restyled. */
const allFiles = Object.entries(paths) as Array<[keyof typeof paths, string]>;

/**
 * Documented exception: `zone_tints` are user-configurable overlay/zone tint
 * defaults from workspace settings, independent of the editor theme (Locked
 * Decision 5). SettingsView keeps the literal fallback array; strip that one
 * line before checking for hardcoded styling hex.
 */
function withoutDocumentedExceptions(source: string): string {
  return source
    .split('\n')
    .filter((line) => !line.includes('zone_tints:'))
    .join('\n');
}

describe('VWP3 views & panels low-chrome treatment', () => {
  test('no hardcoded styling hex in any VWP3 source (zone_tints fallback excepted)', () => {
    for (const [name, path] of allFiles) {
      const matches = withoutDocumentedExceptions(read(path)).match(HEX) ?? [];
      expect(matches, `${name} has hardcoded hex: ${matches.join(', ')}`).toEqual([]);
    }
  });

  test('only VWP2 elevation tokens are used — no arbitrary or built-in heavy shadows', () => {
    const allowed = new Set(['shadow-zf-1', 'shadow-zf-2', 'shadow-zf-3']);
    const shadowUtility =
      /(?<![\w-])((?:[a-z][a-z0-9-]*:)*!?shadow(?:-(?:\[[^\]]+\]|[a-z0-9-]+))?)/g;
    for (const [name, path] of allFiles) {
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
    // The canvas frame and notice use elevation, not a hardcoded ring/shadow.
    expect(read(paths.editor)).not.toContain('shadow-[');
    expect(read(paths.editor)).not.toContain('shadow-xl');
  });

  test('decorative card/row wrappers are removed from views and panels', () => {
    expect(read(paths.home)).not.toContain('rounded-lg border border-zf-border bg-zf-surface');
    expect(read(paths.home)).not.toContain('border-b border-zf-border bg-zf-surface');
    expect(read(paths.home)).not.toContain('overflow-hidden rounded-lg border border-zf-border');

    expect(read(paths.editor)).not.toContain('rounded-md border border-zf-border bg-zf-bg p-0.5');
    expect(read(paths.editor)).not.toContain('border border-zf-border bg-zf-surface shadow-[');

    expect(read(paths.profiles)).not.toContain('border-zf-accent/60');
    expect(read(paths.profiles)).not.toContain('hover:border-zf-border-strong');
    expect(read(paths.profiles)).not.toContain('rounded-md border px-2.5');

    expect(read(paths.library)).not.toContain('rounded-md border border-zf-border bg-zf-surface');
    expect(read(paths.library)).not.toContain('border-b border-zf-border/60');

    expect(read(paths.inspector)).not.toContain('rounded-md border border-zf-border bg-zf-bg');

    expect(read(paths.settings)).not.toContain('rounded-lg border border-zf-border bg-zf-surface');
    // Input borders are intentionally kept; only the callout boxes were de-boxed.
    expect(read(paths.settings)).not.toContain('justify-between rounded-md border border-zf-border bg-zf-bg');

    expect(read(paths.hotkeys)).not.toContain('overflow-hidden rounded-lg border border-zf-border');
    expect(read(paths.hotkeys)).not.toContain('border-b border-zf-border bg-zf-surface');
  });

  test('grouping uses surface contrast + elevation tokens and low-chrome hairlines', () => {
    expect(read(paths.home)).toContain('bg-zf-surface shadow-zf-1');
    expect(read(paths.home)).toContain('border-[color:var(--zf-hairline)]');
    expect(read(paths.home)).toContain("profile.selected ? 'bg-zf-accent/10' : ''");

    expect(read(paths.editor)).toContain('bg-zf-surface shadow-zf-1'); // canvas frame
    expect(read(paths.editor)).toContain('bg-zf-panel px-3 py-2 text-xs text-zf-accent shadow-zf-2'); // notice

    expect(read(paths.profiles)).toContain('bg-zf-accent/10 ring-1 ring-inset ring-zf-accent/50');
    expect(read(paths.profiles)).toContain("'hover:bg-zf-raised'");

    expect(read(paths.library)).toContain('border-[color:var(--zf-hairline)]');
    expect(read(paths.library)).toContain('hover:bg-zf-raised');

    expect(read(paths.inspector)).toContain('bg-zf-bg p-3 shadow-zf-1'); // launcher
    expect(read(paths.inspector)).toContain('leading-relaxed shadow-zf-1'); // log

    expect(read(paths.settings)).toContain('rounded-lg bg-zf-surface p-4 shadow-zf-1');
    expect(read(paths.hotkeys)).toContain('bg-zf-surface shadow-zf-1');
    expect(read(paths.results)).toContain('border-t border-zf-border bg-zf-raised');
  });

  test('meaningful region separators are preserved', () => {
    const editor = read(paths.editor);
    expect(editor).toContain('border-b border-zf-border bg-zf-surface'); // canvas tools
    expect(editor).toContain('border-b border-zf-border bg-zf-bg'); // action bar
    expect(editor).toContain('border-r border-zf-border bg-zf-surface'); // left rail
    expect(editor).toContain('h-2/5 min-h-0 border-b border-zf-border'); // profiles split

    const inspector = read(paths.inspector);
    expect(inspector).toContain('border-l border-zf-border bg-zf-surface'); // pane divider
    expect(inspector).toContain('border-b border-zf-border px-4 py-3'); // header separator

    expect(read(paths.profiles)).toContain('border-b border-zf-border px-3 py-2');
    expect(read(paths.library)).toContain('border-b border-zf-border px-3 py-2');
    expect(read(paths.library)).toContain('border-t border-zf-border p-3');
    expect(read(paths.results)).toContain('border-t border-zf-border');
  });

  test('input/select/textarea borders remain (accessibility affordance)', () => {
    expect(read(paths.settings).match(/border border-zf-border/g)?.length ?? 0).toBeGreaterThanOrEqual(10);
    expect(read(paths.editor)).toContain('border border-zf-border bg-zf-bg'); // profile select + ratio
    expect(read(paths.inspector)).toContain('border border-zf-border bg-zf-surface'); // textarea + key delay
    expect(read(paths.hotkeys)).toContain('<Input');
    expect(read(paths.input)).toContain('border border-border');
    expect(read(paths.select)).toContain('border border-border');
  });

  test('focus rings, keyboard helpers and reduced motion survive in UI primitives', () => {
    const tabs = read(paths.tabs);
    expect(tabs).toContain('role="tablist"');
    expect(tabs).toContain('onTablistKeydown');
    expect(tabs).toContain("case 'ArrowRight'");
    expect(tabs).toContain("case 'Home'");
    expect(tabs).toContain("case 'End'");
    expect(tabs).toContain('focus-visible:outline-2');
    expect(tabs).toContain('motion-reduce:transition-none');

    const toggle = read(paths.switch);
    expect(toggle).toContain('role="switch"');
    expect(toggle).toContain('aria-checked={checked}');
    expect(toggle).toContain('onclick={toggle}');
    expect(toggle).toContain('focus-visible:outline-2');
    expect(toggle).toContain('motion-reduce:transition-none');

    const menu = read(paths.contextMenu);
    expect(menu).toContain('role="menu"');
    expect(menu).toContain('onMenuKeydown');
    expect(menu).toContain("case 'Escape'");
    expect(menu).toContain('focus-visible:outline-2');

    const tip = read(paths.tooltip);
    expect(tip).toContain('role="tooltip"');
    expect(tip).toContain('aria-describedby');
    expect(tip).toContain('isDismissKey');
    expect(tip).toContain('focus-visible:outline-2');

    expect(read(paths.input)).toContain('focus-visible:border-primary');
    expect(read(paths.select)).toContain('focus-visible:border-primary');
    expect(read(paths.resizable)).toContain('motion-reduce:transition-none');
    expect(read(paths.resizable)).toContain('role="separator"');
  });

  test('pane widths and the 2/5 profiles split are unchanged', () => {
    const editor = read(paths.editor);
    expect(editor).toContain('w-60 shrink-0');
    expect(editor).toContain('w-80 shrink-0');
    expect(editor).toContain('h-2/5 min-h-0');
  });

  test('icon-only buttons keep at least a 24x24 target', () => {
    expect(read(paths.profiles).match(/h-6 w-6/g)?.length ?? 0).toBeGreaterThanOrEqual(3);
    expect(read(paths.inspector)).toContain('h-6 w-6');
    expect(read(paths.results)).toContain('h-6 w-6');
  });

  test('AppearanceSettings stays isolated and outside the loaded-draft block', () => {
    const appearance = read(paths.appearance);
    expect(appearance).not.toContain("$lib/state/app");
    expect(appearance).not.toContain("$lib/api");
    expect(appearance).not.toContain('setSettings(');
    expect(appearance).not.toContain('applySettings(');
    expect(appearance).toContain('mode-watcher');
    expect(appearance).toContain('localStorage');
    // Documentation note about preset identities (no behaviour change).
    expect(appearance).toContain('zoneflow-amber');
    expect(appearance).toContain('caffeine');

    const settings = read(paths.settings);
    const appearanceAt = settings.indexOf('<AppearanceSettings />');
    const loadedAt = settings.indexOf('{#if !loaded}');
    expect(appearanceAt).toBeGreaterThan(-1);
    expect(loadedAt).toBeGreaterThan(-1);
    expect(appearanceAt).toBeLessThan(loadedAt);
  });
});
