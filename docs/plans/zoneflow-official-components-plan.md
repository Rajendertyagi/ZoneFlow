# ZoneFlow — Migrate Hand-Rolled UI to Official shadcn-svelte / Bits UI

## Status

- **Plan type**: implementation plan. **This file is documentation only; it changes no application code.**
- **Repository**: `Rajendertyagi/ZoneFlow` — local checkout `D:\Temp\ScreenLayoutRust` (Windows / PowerShell).
- **Baseline**: `main` at **`c6c9eda98a989d4cdbee73a650cc232df5309951`** (`c6c9eda`, `docs(visual): VWP6 verification, contrast report, after-images and docs`, 2026-11-24). Working tree was clean when this plan was written.
- **Plan directory convention**: `docs/plans/` (alongside `zoneflow-visual-polish-plan.md`, `zoneflow-overlay-ux-theme-plan.md`, `zoneflow-feature-preservation.md`).
- **No code changed by this plan**: no `.svelte`, `.ts`, `.css`, `.json`, Rust source, test, or config file was modified to produce this document. No commit and no push were made.
- **Verification status (read this first)**: **No implementation, no automated gate run, no manual browser/keyboard pass, and no screenshot comparison has been performed for this migration.** Every manual item below is marked **NOT YET PERFORMED**. Commands in this plan are instructions to run during implementation, not results already obtained. **No phase acceptance is claimed.**
- **Revision 2 (2026-11-25)**: this plan was revised after a first Phase O1 attempt exposed that the official shadcn-svelte v1.8.1 generator has **runtime prerequisites the original plan did not account for** (see "Official runtime prerequisites" below). The original "no new dependencies / utils.ts unchanged" assumption was wrong and is corrected here. The revision is based on the official manual-install docs, Tailwind v4 docs, and the actual generated files on disk — not guesses.
- **Current disk state (from the aborted O1 attempt)**: `frontend/src/lib/components/ui/` contains 13 generated component directories (`badge`, `button`, `button-group`, `toggle-group`, `context-menu`, `dialog`, `input`, `label`, `native-select`, `switch`, `tabs`, plus CLI-transitive `separator` and `toggle`) alongside the 14 original hand-rolled `.svelte` files and `a11y-helpers.ts`. `frontend/src/app.css` and root `frontend/tsconfig.json` carry the partial O1 additions described below. `bun run check` currently fails with 38 type errors because the official prerequisites are not yet installed. **These are uncommitted working-tree changes at baseline `c6c9eda`; no commit or push was made.** When implementation resumes, reconcile this state with the revised WP-O1 (keep matching generated files, apply the missing prerequisites) rather than starting from a clean tree.

## Why this plan exists

`frontend/src/lib/components/ui/` contains 14 hand-rolled `.svelte` primitives plus `a11y-helpers.ts`. They were introduced during WP3 to avoid adding dependencies, and they work — but they duplicate what the repo already ships: `shadcn-svelte@1.8.1`, `bits-ui@2.22.0`, and `paneforge@1.0.2` are installed, and `components.json` is already configured for the official generator. The hand-rolled copies carry real costs:

1. **Duplicated a11y machinery.** `a11y-helpers.ts` reimplements focus-trap math, roving tabindex, enabled-item navigation and dismiss keys that Bits UI provides natively.
2. **A known modal limitation.** `ui/dialog.svelte` explicitly defers background `inert` and body scroll-lock; the five `dialogs/*` components hand-roll their own scrim/markup with the same gap. Official `Dialog` renders content underneath inert.
3. **API drift.** `tabs.svelte` takes `items[]` + a `children` snippet; official `Tabs` is compositional (`Tabs.Root/List/Trigger/Content`). Every call site must be migrated, not file-swapped.
4. **Dead code.** Six of the 14 primitives have **zero application call sites** — they are referenced only by static tests. They can be deleted outright.

This plan removes as much hand-rolled UI as possible while preserving every frozen decision and the complete theme system.

## Goals

1. **Delete all 14 hand-rolled `.svelte` primitives and `a11y-helpers.ts`.** Eight are replaced by official shadcn-svelte components; six are deleted as unused (no replacement introduced).
2. **Use official composition APIs** (`Tabs.*`, `ContextMenu.*`, `Dialog.*`, `NativeSelect.*`, `ButtonGroup.*`, `ToggleGroup.*`) at every call site, with named imports from `index.js` barrels.
3. **Preserve the theme system intact**: WP1–WP6 JSON presets, generated CSS, `scrim`, app.css-only elevation tokens, per-browser-profile ModeWatcher preferences, `canvas-*` tokens, and `zf-*` aliases.
4. **Preserve frozen behavior**: D1–D6, Rust + Axum + embedded Svelte/Vite architecture, authoritative worker state, native message loop, MoveWindow authority boundary, no new framework/runtime, no backend API or workspace-config changes, no user theme import.
5. **Preserve accessibility**: focus rings, keyboard operation, reduced-motion handling, and ≥24×24 icon targets.
6. **Add only the three official shadcn-svelte runtime prerequisites** — `tailwind-variants`, `cn`, and `tw-animate-css` — and adopt the official `utils.ts` shape. No other dependency is added. (The original "add no dependencies" goal was invalid; see Revision 2.)

### Non-goals

- No visual redesign, no new preset, no change to preset JSON schema or generated CSS values. (Adding `@theme inline` radius tokens, `@custom-variant dark`, and `@import "tw-animate-css"` to `app.css` is required for the official components to render correctly and is **not** a preset/schema change.)
- No change to `Canvas/*` domain components (ZoneCanvas, ZoneRect, SnapGuides, DragPreview, geometry, templates) beyond the one `ContextMenu` call site.
- No change to state ownership, API contracts, dirty-guard logic, or D1–D6 semantics.
- No adoption of Sidebar, Card, Select (non-native listbox), Tooltip, Resizable, Scroll Area, Separator, or Slider where there is no current call site.
- No manual verification is performed by this plan.

## Research basis (verified facts — not re-derived)

**Stack** (`frontend/package.json`): shadcn-svelte `1.8.1` (devDep), bits-ui `2.22.0`, paneforge `1.0.2`, mode-watcher `1.1.0`, tailwindcss `4.3.5`, svelte `5.59.1`, `@lucide/svelte` `1.77.0`, tailwind-merge `^3.7.1`.

**`frontend/components.json`**: baseColor `slate`, css `src/app.css`, aliases `components=$lib/components`, `ui=$lib/components/ui`, `utils=$lib/utils`. It also contains a deprecated `style: "new-york"` field which v1.8.1 ignores (falls back to nova); WP-O1 removes it.

**No `index.ts` barrels exist today.** Official shadcn-svelte uses `frontend/src/lib/components/ui/<name>/index.ts` and named imports such as `import { Button } from '$lib/components/ui/button/index.js';` (namespace imports for multi-part components: `import * as Tabs from '$lib/components/ui/tabs/index.js';`).

**Official docs (shadcn-svelte.com/docs and bits-ui.com/docs, fetched 2026-11-25; spot-checked again for Native Select, Tabs, Context Menu, Dialog, Switch):**

- Official equivalents exist for all 14 hand-rolled components, plus Button Group, Toggle Group, Card, Empty, Sidebar, Native Select, Textarea, Kbd, Spinner, etc.
- `ButtonGroup.Root` / `ButtonGroup.Separator`: `role="group"`, `aria-label`, `orientation`. Docs: "ButtonGroup for actions, ToggleGroup for toggling state." Outline buttons do not need separators (they have borders); other variants do.
- `ToggleGroup.Root` / `ToggleGroup.Item`: the correct official element for state-toggling buttons — Layout/Draw mode and the Gap toggle in `EditorView`.
- `Dialog.Root/Content/Header/Title/Description/Footer/Close`, `showCloseButton={false}`, renders content underneath inert — closes the documented `ui/dialog.svelte` limitation.
- `Tabs.Root/List/Trigger/Content`, `bind:value`, `orientation`, `activationMode`, `loop`, `data-state`, native keyboard nav. `Tabs.Content` defaults `tabindex=0`; set `-1` when the first content is itself focusable.
- `NativeSelect.Root` + `NativeSelect.Option` + `NativeSelect.OptGroup`; all other props pass through to the underlying `<select>` (`bind:value`, `id`, `disabled`, `aria-label`, `onchange`). Docs: "Use NativeSelect for native browser behavior… Use Select for custom styling, animations, or complex interactions."
- `Switch` is a single component (`<Switch bind:checked … />`), not a Root/Thumb pair; `id`, `disabled`, `size`, `aria-label`, `aria-labelledby` pass through.
- `ContextMenu.Root/Trigger/Content/Group/Item/Separator/Shortcut/Sub/…`; `Item` supports `disabled` and `variant="destructive"`.
- Bits UI provides focus trap, roving tabindex, enabled-item navigation and dismiss keys natively — `a11y-helpers.ts` becomes unnecessary once Dialog/Tabs/ContextMenu/Tooltip are official or deleted.
- Official Button: sizes `xs/sm/default/lg` + `icon-xs/sm/default/lg`; variants `default/outline/secondary/ghost/destructive/link`; `href` prop; `Spinner`. Repo button has `sm/default/icon` and `default/outline/ghost/destructive`.
- Tailwind v4 changed button cursor to `default`; official docs recommend `button:not(:disabled), [role="button"]:not(:disabled) { cursor: pointer; }` in `@layer base` if a pointer cursor is wanted.
- shadcn-svelte theming uses CSS vars with background/foreground convention and `@theme inline` mapping. The repo already has the standard tokens plus custom ones (`success`/`warning`, `canvas-*`, `muted-foreground-faint`, `scrim`); official components work with existing tokens. Custom canvas tokens stay in `app.css`.

### Verified during the aborted O1 attempt (2026-11-25)

These are observed facts from running the pinned CLI and reading the generated files — not assumptions:

- **CLI hard-requires `paths` in root `tsconfig.json`.** The repo's root config is solution-style (`files: []`, `references: [...]`) with no `compilerOptions`; the aliases live in `tsconfig.app.json`. `--skip-preflight` does **not** bypass this. An additive `compilerOptions.paths` block in the root config is a required prerequisite (authorized deviation).
- **`style: "new-york"` is deprecated and ignored.** The official schema marks `style` "DEPRECATED IN TAILWIND v4"; v1.8.1 warns `Unsupported style found in components.json: new-york. Using nova instead.` Generated components are **nova-style**. Remove the stale `style` field from `components.json`; keep `baseColor: slate` and all aliases.
- **The CLI generates 13 directories, not 11.** `button-group` and `toggle-group` transitively pull in `separator/` and `toggle/`. These are generated official files (not hand-rolled) and are kept; they have no call sites but are harmless.
- **Generated components import `tv` from `tailwind-variants`** and `cn` plus `WithElementRef` / `WithoutChild` / `WithoutChildrenOrChild` from `$lib/utils.js`. The repo's current `utils.ts` exports only `cn` (a `tailwind-merge` wrapper), producing 38 type errors.
- **Verified prop APIs in the generated files**: `Dialog.Content` has `showCloseButton = true` default (so `showCloseButton={false}` works); `Switch` uses `bind:checked` and forwards `onCheckedChange` via `...restProps`; `NativeSelect.Root` uses `value = $bindable()`; `ButtonGroup` exposes `role="group"`/`orientation`; `ToggleGroup` exposes `value = $bindable()`/`orientation`/`size`/`variant`. All barrels support `import * as X from '…/index.js'` with `X.Root`/`X.Item`/etc.
- **Generated overlay scrim default is `bg-black/10`**, not the `bg-black/50` the earlier plan assumed. The one-line edit replaces `bg-black/10` with `bg-scrim`.
- **Generated imports use correct `$lib/...` aliases** (`$lib/utils.js`, `$lib/components/ui/...`). The blocker is missing exports/deps, not alias paths.

### Official runtime prerequisites (verified from docs + generated files)

The official shadcn-svelte manual-install page requires `tailwind-variants`, `cn`, and `tw-animate-css`, and defines `utils.ts` as `export { cn } from "cn"` plus four local helper types. Tailwind v4 docs confirm `@custom-variant dark` is required for class-based dark mode (the repo uses `:root.dark` via mode-watcher, while Tailwind v4 defaults to `prefers-color-scheme`). The generated nova components use `var(--radius-md)` and `animate-in`/`fade-in-0`/`zoom-in-95` utilities, so the radius scale and `tw-animate-css` import are required.

- `tailwind-variants` and `cn` are already present in `node_modules` transitively; `tw-animate-css` is **not** and must be installed. All three must be declared as direct devDependencies for reproducibility.
- The official `utils.ts` (from shadcn-svelte's own docs app) is:
  ```ts
  export { cn } from "cn";
  export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;
  export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
  export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
  export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
  ```
  This replaces the repo's hand-rolled `cn` wrapper. **`tests/ux-consistency.test.ts` imports `cn` from `../src/lib/utils`** and asserts tailwind-merge behavior (`cn('h-9 w-full', 'h-8 w-52') === 'h-8 w-52'`, falsy values dropped). The `cn` package declares itself a drop-in replacement for clsx + tailwind-merge, so behavior should be compatible, but WP-O1 must run `bun test` (specifically `ux-consistency.test.ts`) to confirm — do not assume zero test impact.
- `app.css` must additionally gain: `@import "tw-animate-css";`, `@custom-variant dark (&:is(.dark *));`, and the radius scale in `@theme inline` (`--radius-sm: calc(var(--radius) * 0.6)`, `--radius-md: calc(var(--radius) * 0.8)`, `--radius-lg: var(--radius)`, `--radius-xl: calc(var(--radius) * 1.4)`, etc.). These are additive and do not change any preset value.

## Migration inventory (14 components + helper)

Call sites were verified by searching all `frontend/src/**/*.svelte|ts`. "Tests only" means no application import exists.

| Hand-rolled file | Official equivalent | Current API → official API | Call sites affected | Risk | Decision & justification |
|---|---|---|---|---|---|
| `ui/button.svelte` | `Button` (+ `ButtonGroup`, `ToggleGroup`) | Default import, `variant`/`size`/`onclick`/snippet → named import, same variant names, `onclick` pass-through, `size` remap (`icon`→`icon-default`); `href` now available | 12 files: `EditorView`, `HomeView`, `HotkeysView`, `SettingsView`, `dialogs/{Close,Confirm,Picker,Prompt,Wizard}Dialog`, `LeftRail/{ProfilesPanel,AppLibrary}`, `Inspector/ZoneInspector`. (`TopBar` uses native `<button>` elements, not this primitive — it is out of scope here.) | Medium (many sites; size/variant remap) | **Migrate.** Highest-value replacement; official is richer and token-compatible. |
| `ui/badge.svelte` | `Badge` | Default import, custom `success`/`warning`/`error` variants → named import + `class` overrides using existing `bg-success/15 text-success` etc. | `TopBar`, `HomeView`, `ApplyResults`, `LeftRail/ProfilesPanel`, `Inspector/ZoneInspector` | Low | **Migrate.** Preserve the four status chips and tinted-pill look via `class`. |
| `ui/input.svelte` | `Input` | `bind:value`, `ariaLabel`, `ariaInvalid`, `ariaDescribedby`, `oninput`/`onchange` → same props pass through on official `Input` | `dialogs/PickerDialog`, `dialogs/PromptDialog`, `Inspector/ZoneInspector`, `LeftRail/AppLibrary`, `HotkeysView`, `SettingsView` | Low | **Migrate.** Near drop-in; keep `border border-border` and focus classes. |
| `ui/label.svelte` | `Label` | `for` + snippet → named `Label` with `for` + snippet | `dialogs/PromptDialog`, `dialogs/WizardDialog`, `Inspector/ZoneInspector`, `AppearanceSettings`, `SettingsView` | Low | **Migrate.** Drop-in. |
| `ui/switch.svelte` | `Switch` | Custom `<button role="switch">` + `onchange` → `<Switch bind:checked … onCheckedChange>` (Bits UI API, verified); `label`/`labelledby` → `aria-label`/`aria-labelledby` | `dialogs/CloseDialog`, `dialogs/PickerDialog`, `dialogs/WizardDialog`, `Inspector/ZoneInspector`, `SettingsView` | Medium (state binding + a11y naming) | **Migrate.** Bits UI owns keyboard/ARIA; preserve contrast-safe thumb via `class`. |
| `ui/tabs.svelte` | `Tabs` | `items[]` + `children(value)` snippet → `Tabs.Root/List/Trigger/Content` composition; `bind:value` retained | `dialogs/PickerDialog`, `Inspector/ZoneInspector`, `LeftRail/AppLibrary` | **High** (API shape change) | **Migrate.** Biggest call-site rewrite; official owns keyboard nav and `data-state`. |
| `ui/context-menu.svelte` | `Context Menu` | `items[]` + `children` snippet → `ContextMenu.Root/Trigger/Content/Item`; `onselect` → `onclick` on `Item`; `disabled` passes through | `Canvas/ZoneCanvas` (1) | Medium | **Migrate.** Removes the last live `a11y-helpers` consumer; Bits UI owns roving focus and dismiss. |
| `ui/select.svelte` | `Native Select` | `options[]` + `value`/`placeholder`/`ariaLabel`/`onchange` → `NativeSelect.Root` + `NativeSelect.Option`; props pass through to `<select>` | `Inspector/ZoneInspector`, `AppearanceSettings`, `SettingsView` | Low–Medium | **Migrate.** Official Native Select preserves native semantics (unlike the non-native `Select` listbox) and is a thin composition. |
| `ui/dialog.svelte` | `Dialog` | Hand-rolled scrim + focus stack → `Dialog.Root/Content/Header/Title/Description/Footer/Close`; `open`/`onclose` → `bind:open` / `Dialog.Close` | **None (tests only)** | Low | **Delete.** No app call site. `dialogs/*` are migrated separately in WP-O4. |
| `ui/resizable.svelte` | `Resizable` (paneforge) | `start`/`end` snippets + pointer math → `Resizable.PaneGroup/Pane/Handle` | **None (tests only)** | Low | **Delete.** Unused; paneforge is installed but no split pane uses it. Do not introduce new usage. |
| `ui/scroll-area.svelte` | `Scroll Area` | `children` snippet → `ScrollArea.Root/Viewport/Scrollbar` | **None (tests only)** | Low | **Delete.** Unused; native `overflow-auto` remains at call sites. |
| `ui/separator.svelte` | `Separator` | `orientation` → `Separator.Root orientation=…` | **None (tests only)** | Low | **Delete.** Unused; region separators are plain borders in views. |
| `ui/slider.svelte` | `Slider` | `value`/`min`/`max`/`step` → `Slider.Root/Thumb` | **None at all** | Low | **Delete.** Fully dead code. |
| `ui/tooltip.svelte` | `Tooltip` | `text` + `children` → `Tooltip.Root/Trigger/Content` | **None (tests only)** | Low | **Delete.** Unused; no tooltip call site to migrate. |
| `ui/a11y-helpers.ts` | *(none — Bits UI provides)* | `trapTabIndex`, `rovingStep`, `edgeIndex`, `FOCUSABLE_SELECTOR`, `isDismissKey`, `enabledIndices`, `nextEnabledIndex`, `edgeEnabledIndex`, `wrapIndex` → native Bits UI behavior | Imported by `context-menu`, `dialog`, `tabs`, `tooltip` + `tests/ui-a11y-helpers.test.ts` | Low after WP-O3/O4 | **Delete** in WP-O5 once all importers are gone. |

**Net result:** 8 components migrated to official equivalents, 6 deleted as unused, 1 helper deleted. `frontend/src/lib/components/ui/` retains only generated official component directories. Domain-specific `Canvas/*` stays custom by design. Note on Dialog: the hand-rolled `ui/dialog.svelte` primitive is deleted (no app call site), but the five `dialogs/*` components are **migrated to the official Dialog** in WP-O4 — Dialog is adopted, not dropped.

### Alternatives considered (and rejected)

- **Non-native `Select` instead of `Native Select`.** Rejected: it is a custom listbox with different keyboard/ARIA semantics and animations. The repo's selects are plain native dropdowns; `Native Select` preserves behavior and is the honest official equivalent.
- **Keep `select.svelte` / `slider.svelte` as documented exceptions.** Considered because they are thin native wrappers and official adds little behavior. Rejected for `select` because official `Native Select` exists and is a small composition; `slider` has zero call sites, so keeping it would preserve dead code rather than remove it. If implementation finds `Native Select` composition cost disproportionate, the fallback is to restore `select.svelte` as a **documented exception** and record it in WP-O6 — this is a deviation requiring explicit re-approval, not a silent choice.
- **Adopt official `Resizable` (paneforge) for the editor split.** Rejected: `resizable.svelte` has no call sites; the editor's split is static CSS (`h-2/5`, `w-60`, `w-80`). Introducing a new pane group would change layout behavior and violate "no behavior change".
- **Adopt `Card` / `Sidebar` / `Tooltip` / `Scroll Area` / `Separator` proactively.** Rejected: no call sites. Installing them would add generated files and token surface with no user-visible benefit. Sidebar token aliases are added only for forward compatibility, not because a component uses them.
- **File-swap hand-rolled components for official ones without updating call sites.** Rejected: the APIs are incompatible (`tabs.svelte` takes `items[]` + snippet; official `Tabs` is compositional). Every call site must be migrated explicitly.
- **Add a DOM test harness (e.g. Vitest + Testing Library) to test component behavior.** Rejected: frozen constraint "no dependency beyond the three official prerequisites (`tailwind-variants`, `cn`, `tw-animate-css`)" and the repo's established text-guard + manual-pass strategy. Behavior is guarded by static assertions and the manual browser pass.

## Migration conventions (apply in every WP)

### 1. CLI install commands

Run from `D:\Temp\ScreenLayoutRust\frontend` (PowerShell). Install only components with real call sites, in one non-interactive command:

```powershell
bun x shadcn-svelte@1.8.1 add -y badge button button-group toggle-group context-menu dialog input label native-select switch tabs
```

The CLI version is **pinned to the installed `1.8.1`** (not `@latest`) so generated APIs match the verified docs. If `1.8.1` is unavailable on the registry, stop and record the deviation before generating files — do not silently fall back to a newer version.

**Prerequisite (authorized deviation):** the CLI hard-requires `compilerOptions.paths` in root `frontend/tsconfig.json`. Add the additive block shown in "Official runtime prerequisites" before running the command. `--skip-preflight` does **not** bypass this check.

Do **not** install `resizable`, `scroll-area`, `separator`, `slider`, `tooltip`, `select` (non-native), `sidebar`, or `card` — there is no call site for them. Note the CLI transitively generates `separator/` and `toggle/` as dependencies of `button-group`/`toggle-group`; keep those generated directories.

**CLI safeguards (WP-O1):** review every generated diff. The CLI must not overwrite `frontend/src/app.css` (theme tokens) or `frontend/src/lib/utils.ts` (which is intentionally replaced with the official shape in the prerequisites step, not by the CLI). If the CLI adds any package beyond the three approved prerequisites, revert that change. If the CLI rewrites `components.json`, keep the existing aliases and `baseColor: slate`, and remove the deprecated `style` field.

### 2. Official runtime prerequisites (WP-O1)

These are **required** for the generated components to type-check and render. They come from the official shadcn-svelte manual-install docs and Tailwind v4 docs, and were confirmed against the generated files on disk.

**a) Declare the three official devDependencies** (all are part of the official setup; `tailwind-variants` and `cn` are already in `node_modules` transitively, `tw-animate-css` is not):

```powershell
bun add -D tailwind-variants cn tw-animate-css
```

No other dependency may be added. If the CLI adds anything else, revert it.

**b) Add the additive `paths` prerequisite to root `frontend/tsconfig.json`** (keep `files: []` and `references` unchanged):

```json
{
  "compilerOptions": {
    "paths": {
      "$lib": ["./src/lib"],
      "$lib/*": ["./src/lib/*"]
    }
  },
  "files": [],
  "references": [ ... unchanged ... ]
}
```

**c) Replace `frontend/src/lib/utils.ts` with the official shape** (note: `tests/ux-consistency.test.ts` imports `cn` from it — verify behavior compatibility with `bun test`):

```ts
export { cn } from "cn";
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
```

**d) Add to `frontend/src/app.css`** (all additive; no existing token value changes):
- `@import "tw-animate-css";` after `@import "tailwindcss";` — provides `animate-in`, `fade-in-0`, `zoom-in-95`, `slide-in-from-*` used by generated components.
- `@custom-variant dark (&:is(.dark *));` - the repo uses `:root.dark` (mode-watcher); Tailwind v4 otherwise defaults to `prefers-color-scheme`, so generated `dark:` utilities would ignore the app's theme toggle. This exact selector is the one used by shadcn-svelte's official manual-install CSS; Tailwind's own docs show the equivalent lower-specificity `&:where(.dark, .dark *)`. Keep `:is` to match the official shadcn-svelte setup; do not mix the two.
- Radius scale in `@theme inline`: `--radius-sm: calc(var(--radius) * 0.6)`, `--radius-md: calc(var(--radius) * 0.8)`, `--radius-lg: var(--radius)`, `--radius-xl: calc(var(--radius) * 1.4)`, `--radius-2xl: calc(var(--radius) * 1.8)`, `--radius-3xl: calc(var(--radius) * 2.2)`, `--radius-4xl: calc(var(--radius) * 2.6)`. Generated nova components reference `var(--radius-md)`.
- The sidebar aliases, cursor rule, and reduced-motion rule described in sections 4 and 6.

**e) Remove the deprecated `style` field from `frontend/components.json`** (schema marks it deprecated in Tailwind v4; v1.8.1 ignores it and falls back to nova). Keep `baseColor: slate` and all aliases unchanged.

### 3. Barrels and import paths

Each `add` creates `frontend/src/lib/components/ui/<name>/index.ts`. Update imports from default `.svelte` imports to named `index.js` imports:

```ts
// before
import Button from '$lib/components/ui/button.svelte';
// after
import { Button } from '$lib/components/ui/button/index.js';

// multi-part components
import * as Tabs from '$lib/components/ui/tabs/index.js';
import * as Dialog from '$lib/components/ui/dialog/index.js';
import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
import * as NativeSelect from '$lib/components/ui/native-select/index.js';
import * as ButtonGroup from '$lib/components/ui/button-group/index.js';
import * as ToggleGroup from '$lib/components/ui/toggle-group/index.js';
```

Never import a generated `.svelte` file directly; always go through `index.js`. **Use one consistent pattern for multi-part components: `import * as X from '…/index.js'` followed by `X.Root` / `X.Item` / `X.Content`.** WP-O1 must open each generated `index.ts` and confirm the actual export shape before any call site is migrated; if a barrel exports a single composed component (e.g. `export { default as ButtonGroup }`), use that named import instead and update every example in this plan to match. Do not mix the two styles.

### 4. Theme token compatibility

- Official components consume `--background`, `--foreground`, `--card`, `--popover`, `--primary`, `--secondary`, `--muted`, `--accent`, `--destructive`, `--border`, `--input`, `--ring` — all already mapped in `app.css` `@theme inline` and emitted per preset in `src/themes/generated/presets.css`. **No preset JSON or generator change is required.**
- Keep custom tokens untouched: `success`/`warning`, `canvas-*`, `muted-foreground-faint`, `scrim`, `zf-*` aliases, and app.css-only `--zf-shadow-*`/`--zf-hairline`.
- **Sidebar tokens**: shadcn-svelte theming defines `--sidebar`, `--sidebar-foreground`, `--sidebar-primary`, `--sidebar-primary-foreground`, `--sidebar-accent`, `--sidebar-accent-foreground`, `--sidebar-border`, `--sidebar-ring`. Sidebar is **not adopted** in this migration, so no component depends on them. For forward compatibility, add aliases in `app.css` `@theme inline` mapped to existing semantic vars (e.g. `--sidebar: var(--card)`, `--sidebar-foreground: var(--card-foreground)`, `--sidebar-primary: var(--primary)`, `--sidebar-accent: var(--secondary)`, `--sidebar-border: var(--border)`, `--sidebar-ring: var(--ring)`). **Do not add these to preset JSON or generated CSS** — that would change the frozen theme schema.
- **Radius scale**: generated nova components reference `var(--radius-md)`; add the `--radius-sm/md/lg/xl/2xl/3xl/4xl` scale in `@theme inline` derived from the existing `--radius` (see section 2d). This is additive and does not change `--radius` itself.
- **Dark mode**: add `@custom-variant dark (&:is(.dark *));` so generated `dark:` utilities follow the app's `:root.dark` class (mode-watcher) instead of the OS `prefers-color-scheme`. Without it, official components would render the wrong theme in dark mode.
- **Animations**: add `@import "tw-animate-css";` so generated `animate-in`/`fade-in-0`/`zoom-in-95`/`slide-in-from-*` utilities exist. Without it those classes silently produce no animation.

### 5. Tailwind v4 cursor rule

Add to `app.css` `@layer base`:

```css
button:not(:disabled),
[role="button"]:not(:disabled) {
  cursor: pointer;
}
```

Because this lives in `@layer base`, Tailwind utilities in later layers still win — the scrim buttons' `cursor-default` utility remains effective. Verify this in the manual pass.

### 6. Focus, keyboard, reduced motion, target size

- Official components ship focus-visible rings (`focus-visible:ring-*`) that differ from the repo's `focus-visible:outline-2 focus-visible:outline-primary`. Preserve the repo convention by passing `class` overrides at call sites (or a small shared class constant) and add static guards that assert the classes are present.
- Add a global reduced-motion rule in `app.css` `@layer base` so official components' internal transitions are covered:

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
  }
}
```

- Keep per-component `motion-reduce:transition-none` where already present.
- Icon-only buttons must keep ≥24×24 targets: use official `size="icon-sm"` (32px) or `size="icon-xs"` (28px) — never smaller. Existing `h-6 w-6` (24px) call sites must be preserved via `class`.

## API migration patterns (before → after)

### Button + ButtonGroup / ToggleGroup

```svelte
<!-- before: EditorView.svelte -->
<script lang="ts">
  import Button from '$lib/components/ui/button.svelte';
</script>
<Button variant="outline" size="sm" onclick={() => setMode('layout')}>Layout</Button>
<Button variant="outline" size="sm" onclick={() => setMode('draw')}>Draw</Button>
<Button variant="ghost" size="icon" onclick={() => (showGap = !showGap)}>…</Button>

<!-- after -->
<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import * as ToggleGroup from '$lib/components/ui/toggle-group/index.js';
</script>
<ToggleGroup.Root type="single" value={mode} onValueChange={(v) => v && setMode(v as 'layout' | 'draw')} aria-label="Canvas mode">
  <ToggleGroup.Item value="layout" size="sm">Layout</ToggleGroup.Item>
  <ToggleGroup.Item value="draw" size="sm">Draw</ToggleGroup.Item>
</ToggleGroup.Root>
<Button variant="ghost" size="icon-sm" aria-pressed={showGap} onclick={() => (showGap = !showGap)}>…</Button>
```

`ToggleGroup` is used for mode/gap **state**; `ButtonGroup` is reserved for action groups (e.g. Undo/Redo/Save in `EditorView`) with `aria-label` and `orientation="horizontal"`. Outline buttons inside a `ButtonGroup` do not need `ButtonGroup.Separator`.

### Tabs (composition migration)

```svelte
<!-- before: AppLibrary.svelte -->
<script lang="ts">
  import Tabs from '$lib/components/ui/tabs.svelte';
  const items = [{ value: 'apps', label: 'Apps' }, { value: 'recent', label: 'Recent' }];
  let tab = $state('apps');
</script>
<Tabs {items} bind:value={tab} label="Library sections">
  {#snippet children(value)}
    {#if value === 'apps'}…{:else}…{/if}
  {/snippet}
</Tabs>

<!-- after -->
<script lang="ts">
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  let tab = $state('apps');
</script>
<Tabs.Root bind:value={tab} class="flex min-h-0 flex-col">
  <Tabs.List aria-label="Library sections">
    <Tabs.Trigger value="apps">Apps</Tabs.Trigger>
    <Tabs.Trigger value="recent">Recent</Tabs.Trigger>
  </Tabs.List>
  <Tabs.Content value="apps" class="min-h-0 flex-1 pt-4">…</Tabs.Content>
  <Tabs.Content value="recent" class="min-h-0 flex-1 pt-4">…</Tabs.Content>
</Tabs.Root>
```

If a panel's first child is focusable, set `tabindex={-1}` on that `Tabs.Content`. Preserve `motion-reduce:transition-none` and focus-ring classes via `class`.

### Context Menu

```svelte
<!-- before: ZoneCanvas.svelte -->
<script lang="ts">
  import ContextMenu from '$lib/components/ui/context-menu.svelte';
  const items = [
    { label: 'Assign…', onselect: () => onpick(zoneId) },
    { label: 'Remove', onselect: () => removeZone(zoneId), disabled: !canRemove },
  ];
</script>
<ContextMenu {items}><div>…</div></ContextMenu>

<!-- after -->
<script lang="ts">
  import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
</script>
<ContextMenu.Root>
  <ContextMenu.Trigger class="focus-visible:outline-2 focus-visible:outline-primary"><div>…</div></ContextMenu.Trigger>
  <ContextMenu.Content>
    <ContextMenu.Item onclick={() => onpick(zoneId)}>Assign…</ContextMenu.Item>
    <ContextMenu.Item disabled={!canRemove} onclick={() => removeZone(zoneId)}>Remove</ContextMenu.Item>
  </ContextMenu.Content>
</ContextMenu.Root>
```

### Dialog (dialogs/* hand-rolled markup → official)

```svelte
<!-- before: ConfirmDialog.svelte -->
{#if request}
  <div class="fixed inset-0 z-[70] flex items-center justify-center p-4">
    <button type="button" aria-label="Cancel dialog" class="absolute inset-0 cursor-default bg-scrim" onclick={() => onresolve(false)}></button>
    <div role="dialog" aria-modal="true" aria-label={request.title} class="relative z-10 w-full max-w-md rounded-lg border border-zf-border-strong bg-zf-panel p-6 shadow-2xl">
      <h2 class="text-base font-semibold text-zf-text">{request.title}</h2>
      <p class="mt-1.5 text-sm text-zf-muted">{request.description}</p>
      <div class="mt-6 flex justify-end gap-2">
        <Button variant="ghost" size="sm" onclick={() => onresolve(false)}>Cancel</Button>
        <Button variant="destructive" size="sm" onclick={() => onresolve(true)}>Confirm</Button>
      </div>
    </div>
  </div>
{/if}

<!-- after -->
<Dialog.Root open={!!request} onOpenChange={(open) => { if (!open) onresolve(false); }}>
  <Dialog.Content showCloseButton={false} class="max-w-md border-zf-border-strong bg-zf-panel shadow-zf-2">
    <Dialog.Header>
      <Dialog.Title class="text-zf-text">{request?.title ?? ''}</Dialog.Title>
      {#if request?.description}<Dialog.Description class="text-zf-muted">{request.description}</Dialog.Description>{/if}
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="ghost" size="sm" onclick={() => onresolve(false)}>Cancel</Button>
      <Button variant="destructive" size="sm" onclick={() => onresolve(true)}>Confirm</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
```

`showCloseButton={false}` is confirmed by the official docs (v1.8.1) and preserves the current "no X button" behavior. **Scrim handling:** the official usage example composes only `Dialog.Root/Content/Header/Title/Description` — `Dialog.Content` renders the overlay internally, so there is no documented prop on `Dialog.Content` for passing a class to the overlay. WP-O1 must open the generated `dialog-content.svelte` and `dialog-overlay.svelte` and apply `bg-scrim` by replacing the generated overlay's default `bg-black/10` with `bg-scrim` (a one-line generated-file edit). Record this as an intentional, minimal deviation in WP-O6; do not leave the scrim as `bg-black/10`, and do not reintroduce a hand-rolled scrim button. Stacked dialogs (wizard → confirm, picker → confirm) must be re-verified: Bits UI handles stacking, but the `z-[70]` override may need to move to `Dialog.Content class="z-[70]"`.

### Switch

```svelte
<!-- before -->
<Switch checked={replace} label="Replace zones" onchange={(v) => (replace = v)} />
<!-- after -->
<script lang="ts">
  import { Switch } from '$lib/components/ui/switch/index.js';
</script>
<Switch id="replace-zones" bind:checked={replace} aria-label="Replace zones" />
<!-- If a call site needs a side effect on toggle, use the Bits UI callback: -->
<Switch id="replace-zones" bind:checked={replace} aria-label="Replace zones" onCheckedChange={(v) => onReplaceChange(v)} />
```

### Native Select

```svelte
<!-- before: AppearanceSettings.svelte -->
<Select options={PRESET_OPTIONS} value={preset} ariaLabel="Theme preset" onchange={(v) => setPreset(v)} />

<!-- after -->
<script lang="ts">
  import * as NativeSelect from '$lib/components/ui/native-select/index.js';
</script>
<NativeSelect.Root aria-label="Theme preset" bind:value={preset}>
  {#each PRESET_OPTIONS as option (option.value)}
    <NativeSelect.Option value={option.value}>{option.label}</NativeSelect.Option>
  {/each}
</NativeSelect.Root>
```

Use **one** state-update mechanism: `bind:value={preset}` is sufficient because `setPreset` already owns the store write at the call site. Do not also attach `onchange` — that double-sets state and assumes `NativeSelect.Root` forwards a native `onchange` event, which must be verified against the generated component in WP-O1 before use. The `placeholder` prop becomes a disabled `<NativeSelect.Option value="" disabled>`. Keep `border border-border` and `focus-visible:border-primary` via `class`.

## Work packages

Sequence is acyclic: **O1 → O2 → O3 → O4 → O5 → O6**. One WP at a time.

### WP-O1 — Foundation: prerequisites, official components, barrels, theme scaffolding

- **Scope** (in this order):
  1. Apply the authorized prerequisite: additive `compilerOptions.paths` in root `frontend/tsconfig.json`.
  2. Declare the three official devDependencies: `bun add -D tailwind-variants cn tw-animate-css`.
  3. Replace `frontend/src/lib/utils.ts` with the official shape (`export { cn } from "cn"` + four helper types).
  4. Remove the deprecated `style` field from `frontend/components.json`; keep `baseColor: slate` and all aliases.
  5. Run the pinned CLI `add -y` command for the 11 components (expect 13 generated dirs including transitive `separator`/`toggle`).
  6. Verify generated `index.ts` barrels, actual export shapes, prop APIs (`showCloseButton`, `onCheckedChange`, `NativeSelect.Root` binding, `ButtonGroup`/`ToggleGroup` composition), and `$lib/...` import aliases.
  7. Apply the one-line `bg-black/10` → `bg-scrim` edit in generated `dialog-overlay.svelte`.
  8. Add to `app.css`: `@import "tw-animate-css"`, `@custom-variant dark`, radius scale, sidebar aliases, cursor rule, reduced-motion rule — all additive, no existing token value changed.
  9. Establish the `index.js` import convention.
- **Non-goals**: no call-site migration, no deletion of hand-rolled files, no behavior change, **no static-test rewrites** (existing tests must keep passing unchanged against the still-present hand-rolled files).
- **Dependencies**: none.
- **Exact files/areas**: `frontend/tsconfig.json` (authorized prerequisite), `frontend/package.json` + `bun.lock` (three approved deps only), `frontend/src/lib/utils.ts` (official shape), `frontend/components.json` (remove deprecated `style`), `frontend/src/app.css` (additive additions), `frontend/src/lib/components/ui/<name>/{index.ts,*.svelte}` (generated, including the scrim edit). **No test files are edited in this WP.**
- **Data flow / state ownership**: unchanged. CLI generates source files; no runtime state.
- **Behavior**: identical UI; only new files, declared deps, and CSS base rules added.
- **Failure / fallback**: if the CLI overwrites `app.css`, restore from baseline `c6c9eda` and re-apply only the intended additions. If the CLI adds any dependency beyond the three approved, revert it. If a generated component does not match the documented API, stop and record the deviation — do not write call-site code against an unverified API. If the CLI still fails at the `paths` check despite the prerequisite, stop and report the exact error.
- **Automated tests**: `bun run check`, `bun test`, `bun run build`, `bun run themes:check` must pass with the new files present **and the existing static tests unchanged** (they still read the hand-rolled files, which remain until their owning WP). `bun run check` passing is the proof that the official prerequisites are complete — the aborted attempt failed here with 38 errors.
- **Manual acceptance (NOT YET PERFORMED)**: none required for this WP beyond a diff review that no preset token value changed.
- **Completion criteria**: 13 generated component directories with barrels; generated APIs/export shapes recorded; `dialog-overlay.svelte` uses `bg-scrim`; `utils.ts` has the official shape; `app.css` contains all additive additions; exactly three new devDependencies; `components.json` has no deprecated `style`; all gates pass with tests untouched.
- **Regression risks**: CLI overwriting theme; accidental extra dependency; barrel import resolution errors; wrong dark variant or missing radius/animation tokens causing silent visual drift.

### WP-O2 — Leaf primitives: Button, Badge, Input, Label, Switch, Native Select

- **Scope**: migrate all call sites of the six leaf components; adopt `ButtonGroup`/`ToggleGroup` where buttons form an action group or a state toggle; delete the six hand-rolled `.svelte` files.
- **Non-goals**: Tabs, Context Menu, Dialog, dead-code cleanup, `a11y-helpers.ts`.
- **Dependencies**: WP-O1.
- **Exact files/areas**: `frontend/src/lib/views/{EditorView,HomeView,HotkeysView,SettingsView}.svelte`; `frontend/src/lib/components/{TopBar,ApplyResults,AppearanceSettings}.svelte`; `frontend/src/lib/components/LeftRail/{ProfilesPanel,AppLibrary}.svelte`; `frontend/src/lib/components/Inspector/ZoneInspector.svelte`; `frontend/src/lib/components/dialogs/{Close,Confirm,Picker,Prompt,Wizard}Dialog.svelte`; delete `ui/{button,badge,input,label,switch,select}.svelte`; in `tests/shell-tokens.test.ts` and `tests/views-tokens.test.ts`, **rewrite only the assertions that read those six deleted files** (see the test-ownership matrix below) — leave all other assertions untouched so they keep passing against still-present hand-rolled files. `StatusBar.svelte` is intentionally not listed: it imports none of these primitives.
- **Data flow / state ownership**: unchanged — `bind:value`/`bind:checked` replace manual `onchange` assignments; state still owned by the same views/stores.
- **Behavior**: identical labels, variants, disabled states, focus rings, ≥24×24 targets, and status chips.
- **Failure / fallback**: if an official variant cannot reproduce a tinted badge, keep the `class` override rather than reintroducing a wrapper. If a size remap changes layout, pass the original `h-7`/`h-9`/`h-6 w-6` via `class`.
- **Automated tests**: `bun run check`, `bun test`, `bun run build`; static guards rewritten to check focus classes, `motion-reduce:transition-none`, four status chip labels, and 24×24 targets at call sites. The existing `app.css` scrim-token mapping guard is left unchanged (component-level `bg-scrim` guards belong to WP-O4).
- **Manual acceptance (NOT YET PERFORMED)**: keyboard Tab/Shift+Tab through every migrated control; focus visible in light/dark; reduced motion; 200% zoom; no layout shift in TopBar/StatusBar.
- **Completion criteria**: zero imports of the six deleted files; all gates pass; static guards updated.
- **Regression risks**: variant/size visual drift; lost `aria-label` on icon buttons; focus-ring style change; `cn()` conflict resolution.

### WP-O3 — Composite primitives: Tabs and Context Menu

- **Scope**: rewrite the three `Tabs` call sites and the one `ContextMenu` call site to composition APIs; delete `ui/tabs.svelte` and `ui/context-menu.svelte`.
- **Non-goals**: Dialog, dead-code cleanup, `a11y-helpers.ts` deletion (still imported by the dead `dialog.svelte`/`tooltip.svelte` until WP-O5).
- **Dependencies**: WP-O1, WP-O2 (call sites co-use Button).
- **Exact files/areas**: `frontend/src/lib/components/dialogs/PickerDialog.svelte`, `frontend/src/lib/components/Inspector/ZoneInspector.svelte`, `frontend/src/lib/components/LeftRail/AppLibrary.svelte`, `frontend/src/lib/components/Canvas/ZoneCanvas.svelte`; delete `ui/tabs.svelte`, `ui/context-menu.svelte`; update `tests/views-tokens.test.ts` — replace the `role="tablist"`/`onTablistKeydown`/`case 'ArrowRight'` assertions with `Tabs.Root`/`Tabs.List`/`Tabs.Trigger` and absence of hand-rolled tablist, **and** replace the `role="menu"`/`onMenuKeydown`/`case 'Escape'` context-menu assertions with `ContextMenu.Root`/`ContextMenu.Content`/`ContextMenu.Item` composition guards and absence of hand-rolled menu markup. Do **not** touch the tooltip `isDismissKey` assertion (owned by WP-O5).
- **Data flow / state ownership**: `bind:value` on `Tabs.Root` replaces the old `value` prop; menu actions still call the same `onselect` equivalents via `ContextMenu.Item onclick`.
- **Behavior**: arrow/Home/End navigation, disabled-item skipping, Escape dismissal, focus restoration — now provided by Bits UI. Preserve `aria-label` on the tablist and the focus ring on the context-menu trigger.
- **Failure / fallback**: if a panel's first child is focusable, set `tabindex={-1}` on `Tabs.Content`. If the canvas right-click coordinates matter, verify `ContextMenu.Trigger` wraps the same element and that `ContextMenu key`/`Shift+F10` still opens it.
- **Automated tests**: `bun run check`, `bun test`, `bun run build`; static guards assert no `role="tablist"`/`role="menu"` hand-rolled markup remains in app code.
- **Manual acceptance (NOT YET PERFORMED)**: keyboard-only tab switching in Picker/Inspector/Library; right-click and `Shift+F10` on canvas; Escape closes without trapping focus; Tab moves on to the next page stop.
- **Completion criteria**: zero imports of `tabs.svelte`/`context-menu.svelte`; all gates pass.
- **Regression risks**: lost roving tabindex; focus trapped in menu; changed tab panel `tabindex`; canvas event interference.

### WP-O4 — Dialogs: migrate `dialogs/*` to official Dialog

- **Scope**: replace hand-rolled scrim/modal markup in all five dialog components with `Dialog.Root/Content/Header/Title/Description/Footer/Close`; delete `ui/dialog.svelte`.
- **Non-goals**: new dialog features, changes to confirm/toast/wizard state logic, D1–D6 behavior changes.
- **Dependencies**: WP-O2, WP-O3 (dialogs use Button/Input/Label/Switch/Tabs).
- **Exact files/areas**: `frontend/src/lib/components/dialogs/{Close,Confirm,Picker,Prompt,Wizard}Dialog.svelte`; delete `ui/dialog.svelte`; update `tests/dialog-tokens.test.ts` (remove `ui/dialog.svelte` path assertions; keep no-hex and generated-CSS `--scrim` guards; retarget the `bg-scrim` guard to the generated `dialog-overlay.svelte` and assert `dialogs/*` contain no hand-rolled scrim button markup).
- **Data flow / state ownership**: `open={!!request}` / `bind:open` replaces `{#if request}`; `onOpenChange(false)` maps to the existing `onclose`/`onresolve(false)` callbacks. No state moves.
- **Behavior**: modal focus trap, background inert, scroll-lock, Escape dismissal, focus restoration — now native to Bits UI. `showCloseButton={false}` preserves the current no-X look. `bg-scrim` must remain on the generated overlay (applied in WP-O1), not on a hand-rolled scrim button.
- **Failure / fallback**: if stacked dialogs (wizard → confirm, picker → confirm) fight over focus, set an explicit `class="z-[70]"` on the inner `Dialog.Content` and re-audit; do not reintroduce a hand-rolled focus stack. If `Dialog.Content` default padding/spacing differs, pass `class` overrides rather than changing theme tokens.
- **Automated tests**: `bun run check`, `bun test`, `bun run build`; `dialog-tokens.test.ts` updated and passing; static guards that the generated `dialog-overlay.svelte` uses `bg-scrim` and no `bg-black/*` default remains, and that `dialogs/*` contain no hand-rolled scrim button (`bg-scrim`/`bg-black/*` on a `<button>`) and no `role="dialog"` markup.
- **Manual acceptance (NOT YET PERFORMED)**: open/close every dialog by mouse and keyboard; Escape; focus returns to trigger; background is inert and does not scroll; stacked confirm dialogs behave; scrim readable in all presets/modes.
- **Completion criteria**: zero hand-rolled `role="dialog"`/`bg-scrim` button markup in `dialogs/*`; `ui/dialog.svelte` deleted; all gates pass.
- **Regression risks**: focus restoration to a stale trigger; z-index stacking; scroll-lock side effects; loss of `aria-label`/`aria-describedby`.

### WP-O5 — Dead-code removal and `a11y-helpers.ts` deletion

- **Scope**: delete the five remaining unused primitives (`resizable`, `scroll-area`, `separator`, `slider`, `tooltip`) and the `a11y-helpers.ts` helper; remove/rewrite their static tests. (`dialog.svelte`, the sixth unused primitive, was already deleted in WP-O4.)
- **Non-goals**: no new component adoption, no call-site changes (there are none).
- **Dependencies**: WP-O4 (so `a11y-helpers.ts` has no remaining importers).
- **Exact files/areas**: delete `ui/{resizable,scroll-area,separator,slider,tooltip}.svelte` and `ui/a11y-helpers.ts`; delete `tests/ui-a11y-helpers.test.ts`; update `tests/shell-tokens.test.ts` (remove `ui/separator.svelte` path and its hairline assertions, or retarget to the plain border utilities in views); update `tests/views-tokens.test.ts` (remove `tooltip`/`resizable`/`scroll-area` path entries and their assertions).
- **Data flow / state ownership**: none — deleted files had no consumers.
- **Behavior**: unchanged.
- **Failure / fallback**: if any deleted file turns out to be imported (re-run the search), restore it and convert it to a documented exception rather than breaking the build.
- **Automated tests**: full `bun run check`, `bun test`, `bun run build`, `bun run themes:check`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **Manual acceptance (NOT YET PERFORMED)**: none specific; rely on WP-O2/O3/O4 manual passes.
- **Completion criteria**: `frontend/src/lib/components/ui/` contains only generated official component directories; no test references a deleted path; all gates pass.
- **Regression risks**: stale static-test references; accidental deletion of a file with a dynamic import.

### WP-O6 — Final verification, documentation and exceptions register

- **Scope**: full gate run, theme-token audit, a11y static assertions, update docs, record any documented exceptions.
- **Non-goals**: no new code changes beyond test/doc adjustments needed to make gates pass.
- **Dependencies**: WP-O5.
- **Exact files/areas**: `frontend/tests/*`, `docs/manual-verification-checklist.md` (add a new unchecked section for this migration), this plan file (append a factual status record only after implementation), `docs/theme-and-appearance.md` (note sidebar aliases if added).
- **Data flow / state ownership**: unchanged.
- **Behavior**: unchanged.
- **Failure / fallback**: any failing gate blocks completion; fix in the owning WP and re-audit before re-running O6.
- **Automated tests**: `bun install --frozen-lockfile`, `bun run check`, `bun test`, `bun run build`, `bun run themes:check`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.
- **Manual acceptance (NOT YET PERFORMED)**: full browser a11y pass, all four presets × two modes, 200% zoom, reduced motion, D1–D6 journey (Open → Home → Edit → Assign → Save → Apply → Launch & Arrange → Tray switch → Disable → Enable).
- **Completion criteria**: every acceptance criterion in the next section has a recorded verification method and result; manual items explicitly marked NOT YET PERFORMED if not run.
- **Regression risks**: false "all green" claims from skipping manual items; theme drift from CLI-generated CSS.

## Test migration strategy

The existing static tests are **text-based guards** (no DOM harness; no dependency beyond the three official prerequisites). They must be rewritten, not simply deleted:

| Test | Current guard | Migration action |
|---|---|---|
| `tests/ui-a11y-helpers.test.ts` | Tests `trapTabIndex`, `rovingStep`, `FOCUSABLE_SELECTOR`, etc. | **Delete** — Bits UI owns this behavior. Replace with call-site assertions in `views-tokens.test.ts` that no hand-rolled `role="tablist"`/`role="menu"` remains. |
| `tests/dialog-tokens.test.ts` | Reads `ui/dialog.svelte` for `bg-scrim`, no hex, no stale comment | Retarget `bg-scrim` to generated `dialog-overlay.svelte`; assert `dialogs/*` have no hand-rolled scrim button; keep generated-CSS `--scrim` and `@theme inline` mapping guards. |
| `tests/shell-tokens.test.ts` | Reads `ui/{button,badge,input,separator}.svelte` | Retarget button/badge/input to generated official sources; remove separator path or retarget to view border utilities; keep focus-ring, disabled, reduced-motion, status-chip and 24×24 guards. |
| `tests/views-tokens.test.ts` | Reads `ui/{tabs,input,select,switch,tooltip,context-menu,resizable,scroll-area}.svelte` | Retarget to generated official sources + call sites; remove tooltip/resizable/scroll-area entries; replace keyboard-helper assertions with composition-API assertions. |
| `tests/canvas-tokens.test.ts` | Canvas-only | **Unchanged** — `Canvas/*` stays custom. |

**Note on `utils.ts`**: `tests/ux-consistency.test.ts` imports `cn` from `../src/lib/utils` and asserts tailwind-merge conflict resolution and falsy dropping. Replacing `utils.ts` with the official shape (`export { cn } from "cn"`) should preserve that behavior because the `cn` package is a drop-in replacement for clsx + tailwind-merge, but WP-O1 must run `bun test` (including `ux-consistency.test.ts`) to confirm — no test file is edited, but the test is a consumer whose behavior must keep passing. The `app.css` guards in `dialog-tokens.test.ts` and `shell-tokens.test.ts` assert only the scrim mapping and elevation tokens, which the additive WP-O1 changes do not alter.

#### Test-ownership matrix (avoids conflicting rewrites of shared test files)

Several test files assert against multiple hand-rolled components that are deleted in different WPs. Each WP owns **only** the assertions for the files it deletes; all other assertions must keep passing against still-present hand-rolled files until their owning WP. No WP rewrites a whole shared test file at once.

| Test file | Assertion group | Owning WP | Action in that WP |
|---|---|---|---|
| `shell-tokens.test.ts` | `ui/button.svelte`, `ui/badge.svelte`, `ui/input.svelte` focus/disabled/reduced-motion/status-chip/24×24 guards | WP-O2 | Retarget to generated official sources + call sites |
| `shell-tokens.test.ts` | `ui/separator.svelte` path + hairline assertions | WP-O5 | Remove path or retarget to plain border utilities in views |
| `views-tokens.test.ts` | `ui/input.svelte`, `ui/select.svelte`, `ui/switch.svelte` path/prop assertions | WP-O2 | Retarget to generated official sources + call sites |
| `views-tokens.test.ts` | `ui/tabs.svelte` `role="tablist"` / `onTablistKeydown` / arrow-key assertions; `ui/context-menu.svelte` `role="menu"` / `onMenuKeydown` / `case 'Escape'` / focus assertions | WP-O3 | Replace with `Tabs.*` / `ContextMenu.*` composition-API assertions and absence of hand-rolled markup |
| `views-tokens.test.ts` | `ui/tooltip.svelte` (incl. `isDismissKey`), `ui/resizable.svelte`, `ui/scroll-area.svelte` path entries and assertions | WP-O5 | Remove entries and assertions |
| `dialog-tokens.test.ts` | `ui/dialog.svelte` path assertions; `bg-scrim` / no-hex / `--scrim` guards | WP-O4 | Remove path assertions; assert `bg-scrim` on generated `dialog-overlay.svelte` and absence of hand-rolled scrim buttons in `dialogs/*` |
| `ui-a11y-helpers.test.ts` | whole file | WP-O5 | Delete; replace coverage with call-site assertions in `views-tokens.test.ts` (WP-O3) |
| `canvas-tokens.test.ts` | whole file | none | Unchanged in every WP |

**Rule:** after each WP, `bun test` must pass with only that WP's owned assertion groups changed. If a group fails because its source file was deleted early, the WP sequence is wrong — stop and re-audit rather than rewriting unrelated assertions.

**Component behavior tests**: no DOM harness exists and none is added. Behavior is guarded by (a) static source assertions and (b) the manual browser pass. Do not claim DOM-level test coverage that does not exist.

**Manual browser a11y pass — NOT YET PERFORMED**: keyboard-only traversal, focus visibility in both modes, Escape/dismiss behavior, reduced motion, 200% zoom, and contrast in all four presets × two modes. Record date, OS/browser build and observations when actually performed.

**Screenshot comparison — NOT YET PERFORMED**: if performed, reuse the `docs/screenshots/visual-polish/fixture.md` setup and compare against the `after-*.png` baseline; record deviations rather than claiming pixel identity.

## Acceptance criteria (measurable, with verification method)

| # | Criterion | Verification method |
|---|---|---|
| AC1 | No hand-rolled equivalents remain: `frontend/src/lib/components/ui/` contains only generated official component directories; `a11y-helpers.ts` deleted. Documented exceptions: none (Canvas/* was never in scope). | `Get-ChildItem -Recurse frontend/src/lib/components/ui` + `git status`; static test asserting no `ui/*.svelte` at the directory root. |
| AC2 | All call sites compile with named `index.js` imports; zero imports of deleted files. | `bun run check` (0 errors/0 warnings) + search of **app code only** (exclude `frontend/src/lib/components/ui/**`) for `$lib/components/ui/*.svelte` — generated internals legitimately import local `.svelte` files and must not count. |
| AC3 | Frontend gates pass. | `bun install --frozen-lockfile`, `bun test` (0 fail), `bun run build`, `bun run themes:check`. |
| AC4 | Backend gates pass; no Rust/API/config change. | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`; `git diff --name-only` shows no `src/**/*.rs` or workspace config. |
| AC5 | Only the three official prerequisites added: `tailwind-variants`, `cn`, `tw-animate-css`. No other dependency. | `git diff frontend/package.json bun.lock` shows exactly those three devDependencies and no others; CLI-generated files only. |
| AC6 | Theme system intact: preset JSON, generated CSS, `scrim`, elevation tokens, canvas tokens, `zf-*` aliases, ModeWatcher prefs unchanged. | `bun run themes:check`; `tests/{dialog-tokens,shell-tokens,views-tokens,canvas-tokens}.test.ts` pass; `git diff` on `src/themes/**` shows no changes; `app.css` diff shows only the additive items in WP-O1 (tw-animate import, dark variant, radius scale, sidebar aliases, cursor rule, reduced-motion rule). |
| AC12 | Official runtime prerequisites complete: `utils.ts` has the official shape; `tsconfig.json` has the additive `paths`; `components.json` has no deprecated `style`; generated components type-check. | `bun run check` passes with 0 errors (the aborted attempt failed here with 38 errors); inspect `utils.ts`, root `tsconfig.json`, and `components.json` diffs. |
| AC7 | Focus rings, keyboard operation, reduced motion and ≥24×24 targets preserved. | Static guards in `views-tokens.test.ts`/`shell-tokens.test.ts` + **manual browser pass (NOT YET PERFORMED)**. |
| AC8 | D1–D6 behavior unchanged. | `cargo test` + manual D1–D6 journey (**NOT YET PERFORMED**); no diff in `src/**/*.rs` or state/API files. |
| AC9 | Dialogs are modal with inert background and scroll-lock; stacked dialogs work. | Static `bg-scrim` guard + **manual pass (NOT YET PERFORMED)**. |
| AC10 | Tabs/ContextMenu keyboard nav and dismissal work via Bits UI. | Static composition-API guards + **manual pass (NOT YET PERFORMED)**. |
| AC11 | Every manual item is honestly reported. | Final report marks unrun items **NOT YET PERFORMED**; no phase acceptance claimed until each WP's criteria are evidenced. |

## Risks, non-goals and safeguards

| Risk | Safeguard |
|---|---|
| CLI overwrites `app.css` or adds an unapproved dependency | WP-O1 diff review; restore from `c6c9eda`; only the three approved deps allowed (AC5). `utils.ts` is replaced intentionally with the official shape, not by the CLI. |
| Generated components fail `bun run check` (38 errors in the aborted attempt) | Install the three official prerequisites and adopt the official `utils.ts` shape in WP-O1; `bun run check` is the completion gate (AC12). |
| Generated `dark:` utilities ignore the app theme toggle | Add `@custom-variant dark (&:is(.dark *));` in WP-O1; verify in both modes in the manual pass. |
| Generated `var(--radius-md)` / animation classes silently produce no style | Add the radius scale and `@import "tw-animate-css"` in WP-O1; verify visually in the manual pass. |
| Nova-style visual drift vs the repo's current look | Pass `class` overrides at call sites; screenshot comparison if performed; no preset value changes. |
| Official focus-ring style differs from repo convention | Pass `class` overrides; static guards assert `focus-visible:outline-2` / `focus-visible:outline-primary`. |
| Tailwind v4 default cursor changes button feel | Add `@layer base` pointer rule; verify scrim `cursor-default` still wins. |
| Reduced-motion regressions inside official components | Global `@media (prefers-reduced-motion: reduce)` rule + manual pass. |
| Tab panel focus order changes | Set `tabindex={-1}` on `Tabs.Content` when first child is focusable. |
| Stacked dialogs fight over focus | Use `Dialog.Content class="z-[70]"`; do not reintroduce hand-rolled focus stack. |
| Static tests become meaningless after source deletion | Rewrite guards to assert call-site composition APIs and absence of hand-rolled markup. |
| Visual drift in badges/buttons | Preserve tinted-pill and shadow utilities via `class`; screenshot comparison if performed. |
| Scope creep into Canvas/* or theme schema | Non-goals above; `canvas-tokens.test.ts` unchanged; no preset JSON edits. |

**Explicit non-goals**: no visual redesign, no new framework/runtime, no backend API or workspace-config change, no user theme import, no dependency beyond the three official prerequisites, no manual verification performed by this plan.

## Execution model

- **One WP at a time**, strictly in order O1 → O2 → O3 → O4 → O5 → O6. No parallel WPs; no shared-state conflicts.
- **Single subagent at a time**: code by `opencode/hy4-flash-free`, audit by `opencode/iceman-free`. Do not run code and audit agents concurrently.
- **Fix → re-audit until clean**: an audit finding returns the WP to the code agent; re-audit the same WP before advancing.
- **Commit + push per checkpoint**: each WP that passes its completion criteria is committed and pushed following the repo's direct-to-main workflow (CI runs on push). No commit or push is made by this planning task.
- **Automated vs manual reporting honesty**: automated gate results are reported as commands run with outcomes; manual items are reported as **NOT YET PERFORMED** until an interactive session actually completes them. Passing automated checks are never presented as evidence for manual acceptance.

## Verification commands reference (PowerShell, from repo root)

```powershell
# WP-O1 prerequisite install (run ONCE; modifies package.json + bun.lock)
cd frontend
bun add -D tailwind-variants cn tw-animate-css   # exactly three deps, no others

# Frontend gates (run after the lockfile is updated and committed)
cd frontend
bun install --frozen-lockfile                    # clean verification against the committed lockfile
bun run check                                    # must be 0 errors (was 38 before prerequisites)
bun test
bun run build
bun run themes:check

# Backend gates
cd ..
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release

# Inventory / no-dead-import checks
Get-ChildItem -Recurse -File frontend/src/lib/components/ui | Select-Object -ExpandProperty FullName
Get-ChildItem -Recurse -File frontend/src -Include *.svelte,*.ts |
  Where-Object { $_.FullName -notmatch '\\lib\\components\\ui\\' } |
  Select-String -Pattern '\$lib/components/ui/.*\.svelte'   # must return nothing (app code only)
git diff --name-only                                        # must show no src/**/*.rs or config changes
```

The advisory `cargo audit` and release-artifact upload steps are **not** part of these gates (consistent with the VWP6 baseline).

## Explicit statement

**This plan does NOT implement anything.** It changes no application code, no tests, no configuration, and no theme files. **Manual verification has not been performed.** **No phase acceptance is claimed.** All implementation, automated gate runs, manual browser/keyboard passes, commits and pushes are future work to be carried out under the execution model above.

---

## Implementation status record (appended by WP-O6)

This section is a **factual record only**. It does not change the plan body
above, does not claim phase acceptance, and does not authorize a commit or push.

- **Record date**: 2026-11-25.
- **Baseline HEAD**: `c6c9eda98a989d4cdbee73a650cc232df5309951`
  (`c6c9eda`, `docs(visual): VWP6 verification, contrast report, after-images and docs`, 2026-11-24).
- **Working tree at record time**: all O1–O6 changes are uncommitted on top of
  the baseline; no commit and no push was made by this record.

### Phases completed

- **WP-O1** (foundation: prerequisites, official components, barrels, theme
  scaffolding) — completed; audited CLEAN per handoff.
- **WP-O2** (leaf primitives: Button, Badge, Input, Label, Switch, Native Select)
  — completed; audited CLEAN per handoff.
- **WP-O3** (composite primitives: Tabs and Context Menu) — completed; audited
  CLEAN per handoff.
- **WP-O4** (dialogs: `dialogs/*` migrated to official Dialog) — completed;
  audited CLEAN per handoff.
- **WP-O5** (dead-code removal and `a11y-helpers.ts` deletion) — completed;
  audited CLEAN per handoff.
- **WP-O6** (final verification, documentation and exceptions register) — this
  record. Full gate suite run from repo root on 2026-11-25; exact outcomes below.

### Full gate suite results (run in WP-O6, from repo root)

Frontend gates (from `frontend/`):

| Command | Outcome |
|---|---|
| `bun install --frozen-lockfile` | PASS — `Checked 89 installs across 135 packages (no changes)` |
| `bun run check` | PASS — `svelte-check found 0 errors and 0 warnings` |
| `bun test` | PASS — `249 pass, 0 fail, 1847 expect() calls` across 24 files |
| `bun run build` | PASS — Vite production build succeeded (5447 modules transformed) |
| `bun run themes:check` | PASS — `generated/presets.css is current` |

Backend gates (from repo root):

| Command | Outcome |
|---|---|
| `cargo fmt --check` | PASS — no output, exit 0 |
| `cargo clippy --all-targets -- -D warnings` | PASS — `Finished dev profile`, no warnings |
| `cargo test` | PASS — 264 unit tests + 3 integration tests, 0 failed |
| `cargo build --release` | PASS — `Finished release profile` |

Inventory checks:

| Check | Outcome |
|---|---|
| `Get-ChildItem -Recurse -File frontend/src/lib/components/ui` | PASS — only generated component directories (`badge`, `button`, `button-group`, `context-menu`, `dialog`, `input`, `label`, `native-select`, `separator`, `switch`, `tabs`, `toggle`, `toggle-group`); no root-level `.svelte`/`.ts` files |
| App-code search for `$lib/components/ui/*.svelte` excluding `frontend/src/lib/components/ui/**` | PASS — no matches |
| `git diff --name-only` | PASS — no `src/**/*.rs` or workspace-config changes |

Post-documentation-edit re-run (to confirm doc-only changes broke nothing):
`bun run check` PASS (0 errors/0 warnings), `bun test` PASS (249 pass/0 fail),
`bun run build` PASS, `bun run themes:check` PASS, `cargo fmt --check` PASS,
`cargo clippy --all-targets -- -D warnings` PASS, `cargo test` PASS.

### Documented deviations (all intentional, recorded at implementation time)

1. **tsconfig paths prerequisite** — an additive `compilerOptions.paths` block
   was added to root `frontend/tsconfig.json` (the CLI hard-requires it;
   `--skip-preflight` does not bypass it). `files: []` and `references` unchanged.
2. **Three official dependencies** — exactly `tailwind-variants`, `cn` and
   `tw-animate-css` were added as devDependencies; no other dependency was added.
3. **Nova style** — the deprecated `style: "new-york"` field was removed from
   `components.json`; v1.8.1 ignores it and generates nova-style components.
4. **Transitive `separator/` and `toggle/` directories** — generated by the CLI
   as dependencies of `button-group`/`toggle-group`. They are official generated
   files with no application call sites; kept and documented, not hand-rolled.
5. **`bg-black/10` → `bg-scrim`** — the generated `dialog-overlay.svelte` default
   scrim class was replaced with `bg-scrim` (one-line generated-file edit); no
   hand-rolled scrim button was reintroduced.
6. **`onSelect` instead of the plan's `onclick` snippet** — the Context Menu
   call site in `Canvas/ZoneCanvas.svelte` uses the existing `onselect` item
   descriptor shape rather than the plan's illustrative `onclick` snippet; the
   official `ContextMenu.Item` composition is used.
7. **ContextMenu `bind:open` instead of a synthetic event** — keyboard opening
   (`ContextMenu` key / `Shift+F10`) uses the official bindable `open` state on
   `ContextMenu.Root`; no synthetic `contextmenu` event is dispatched.
8. **PromptDialog Enter guard** — the prompt-specific Enter shortcut is kept via
   a `<svelte:window onkeydown>` handler that early-returns when no request is
   open and scopes Enter away from interactive controls.
9. **ConfirmDialog `aria-label` removal** — the redundant `aria-label` on the
   dialog container was removed; the official `Dialog.Title` provides the
   accessible name.

### Documented exceptions register

- **Hand-rolled equivalents**: none remain. All 14 hand-rolled `.svelte`
  primitives and `a11y-helpers.ts` were deleted (8 migrated to official
  equivalents, 6 deleted as unused, 1 helper deleted).
- **Generated `separator/` and `toggle/` directories**: CLI-transitive, no call
  sites — kept and documented (deviation 4 above).
- **`Canvas/*`**: remains custom by design; it was never in scope. Only the one
  `ContextMenu` call site in `ZoneCanvas.svelte` was migrated.

### Acceptance criteria results (AC1–AC12)

| # | Result | Verification method | Observed result |
|---|---|---|---|
| AC1 | PASS | `Get-ChildItem -Recurse -File frontend/src/lib/components/ui` + `git status` + static test | `ui/` contains only 13 generated official directories; no root-level files; all 14 hand-rolled `.svelte` files and `a11y-helpers.ts` deleted; no documented hand-rolled exceptions. |
| AC2 | PASS | `bun run check` + app-code-only search excluding `frontend/src/lib/components/ui/**` | 0 errors/0 warnings; no app-code import of `$lib/components/ui/*.svelte`. |
| AC3 | PASS | `bun install --frozen-lockfile`, `bun test`, `bun run build`, `bun run themes:check` | All four passed (see gate table). |
| AC4 | PASS | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`; `git diff --name-only` | All four passed; no `src/**/*.rs` or workspace-config changes. |
| AC5 | PASS | `git diff frontend/package.json frontend/bun.lock` | Exactly three devDependencies added (`cn`, `tailwind-variants`, `tw-animate-css`); no others. |
| AC6 | PASS | `bun run themes:check`; token tests; `git diff` on `src/themes/**`; `app.css` diff | `themes:check` current; no `src/themes/**` changes; `app.css` diff contains only the additive WP-O1 items (tw-animate import, dark variant, radius scale, sidebar aliases, cursor rule, reduced-motion rule). |
| AC12 | PASS | `bun run check` + inspect `utils.ts`, root `tsconfig.json`, `components.json` diffs | 0 errors; `utils.ts` has official shape; `tsconfig.json` has additive `paths`; `components.json` has no deprecated `style`. |
| AC7 | PARTIAL — static PASS, manual **NOT YET PERFORMED** | Static guards in `views-tokens.test.ts`/`shell-tokens.test.ts` + manual browser pass | Static guards pass; keyboard/focus/reduced-motion/target-size behaviour in a real browser not verified. |
| AC8 | PARTIAL — automated PASS, manual **NOT YET PERFORMED** | `cargo test` + manual D1–D6 journey; no Rust/state/API diff | 264 + 3 Rust tests pass; no `src/**/*.rs` diff; D1–D6 journey not walked in a browser. |
| AC9 | PARTIAL — static PASS, manual **NOT YET PERFORMED** | Static `bg-scrim` guard + manual pass | `dialog-overlay.svelte` uses `bg-scrim`; modal inert/scroll-lock/stacked-dialog behaviour not verified interactively. |
| AC10 | PARTIAL — static PASS, manual **NOT YET PERFORMED** | Static composition-API guards + manual pass | `Tabs.*`/`ContextMenu.*` composition guards pass; keyboard nav/dismissal not verified interactively. |
| AC11 | PASS | Final report honesty | All unrun manual items are marked **NOT YET PERFORMED** in the manual checklist and this record; no phase acceptance claimed. |

### Explicit statement

**Manual verification is NOT YET PERFORMED.** No browser, keyboard, visual,
screenshot, 200%-zoom, reduced-motion, contrast, D1–D6 journey, stacked-dialog,
context-menu-keyboard, tabs-keyboard or scrim-readability pass was carried out
for this migration. The automated gate results above are evidence only for the
automated/static criteria. **No phase acceptance is claimed by this record.**
No commit and no push were made.
