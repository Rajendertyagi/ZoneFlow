# ZoneFlow Studio — GUI Architecture

**Status:** Planning document. No implementation changes are authorized by this file.
**Date:** 2026-11-21
**Basis:** GUI architecture audit of all 13 `src/editor/*.rs` files + egui 0.37 / shadcn-egui 0.8 source verification.

---

## 1. Current GUI structure (verified)

All UI lives in `src/editor/` as `impl Editor` blocks. `Editor` (`mod.rs:34-102`) is the single state container.

| File | Lines | Responsibility |
|---|---:|---|
| `mod.rs` | ~1,280 | Editor struct, config/persistence, zone mutations, shortcuts, shell (`body`/`center`/`ui`), `eframe::App` impl |
| `canvas.rs` | ~1,047 | Hand-painted canvas: interaction, painting, geometry, merge |
| `rail.rs` | ~862 | App enumeration/cleanup (~550 lines) + left panel UI |
| `dialogs.rs` | ~707 | Six shadcn Dialog modals + toast |
| `inspector.rs` | ~432 | Right panel, 3 tabs, console |
| `state.rs` | ~290 | Pure state types, templates, presets |
| `views.rs` | ~187 | Hotkeys + Settings views |
| `toolbar.rs` | ~194 | Canvas toolbar |
| `action_bar.rs` | ~145 | Bottom action bar + apps strip |
| `theme.rs` | ~132 | Canvas palette tokens |
| `top_bar.rs` | ~59 | Brand/profile/resolution/dirty + "Engine ready" |
| `history.rs` | ~70 | Undo/redo |
| `tokens.rs` | ~84 | Layout/color constants |

### Current shell (exists but flawed)
`ui()` (`mod.rs:1237-1279`) is a flat sequence: shortcuts → top_bar → body → 6 modals → toast → overlay. `body()` (`mod.rs:909-945`) stacks three native egui panels (left, right only in Canvas view, CentralPanel last). `center()` (`mod.rs:947-960`) stacks toolbar → canvas → action bar → apps strip.

**Problem:** no component owns exact region rectangles. `canvas_view()` guesses its height as `available_size().y - settings.stage_lower_h` (`canvas.rs:43`); `rail.rs:595` guesses `available_height() - 56.0`. Siblings cannot know each other's real size → clipping when maximized, wasted space otherwise.

---

## 2. Proposed application shell

### 2.1 Shell component
Create a single `Shell` owner (new module `src/editor/shell.rs`) that divides the viewport into exact regions and hands rects down. **Do not use `egui::StripBuilder` — it does not exist in egui 0.37** (verified in crate source). Use `egui::Panel::{top,bottom,left,right}` + explicit rect math (`allocate_ui_with_layout`, `available_rect_before_wrap`).

**Design priority:** the canvas gets the maximum available space. The application library is easily accessible but does not permanently consume width. The inspector is contextual. Secondary settings never compete permanently with the canvas. **Do not force a three-column layout at all times** — the right aside appears only when needed.

```
Editor — canvas-focused (default, no selection):
┌──────────────────────────────────────────────────────────────┐
│ TopBar: brand · Editing: Website · [SAVED] · [4K ▾]           │
├────────────┬─────────────────────────────────────────────────┤
│ LeftRail   │ Toolbar: [Layout][Draw][Undo][Redo][Split]...    │
│ (toggle)   ├─────────────────────────────────────────────────┤
│ Apps       │                                                 │
│ [Search]   │              CANVAS (maximum space)             │
│ ...        │                                                 │
│            ├─────────────────────────────────────────────────┤
│ Profiles   │ Footer: [Save] [Revert]      [Apply]             │
│ [Tools]    │ Apps in this workspace (compact, scroll)         │
├────────────┴─────────────────────────────────────────────────┤
│ Status: Ready · 3 zones · 2 assigned · Ctrl+drag to snap      │
└──────────────────────────────────────────────────────────────┘

Editor — with zone selected (inspector appears contextually):
┌──────────────────────────────────────────────────────────────┐
│ TopBar: brand · Editing: Website · [UNSAVED ●] · [4K ▾]       │
├────────────┬───────────────────────────────┬─────────────────┤
│ LeftRail   │ Toolbar                       │ INSPECTOR       │
│ (collaps.) ├───────────────────────────────┤ (contextual)    │
│ Apps       │                               │ Zone 1          │
│ [Search]   │        CANVAS (smaller)        │ Label / Exe     │
│ ...        │                               │ Rules           │
│            ├───────────────────────────────┤ [Console ▾]     │
│ Profiles   │ Footer: [Save][Revert][Apply]  │                 │
├────────────┴───────────────────────────────┴─────────────────┤
│ Status: Ready · 3 zones · 2 assigned                          │
└──────────────────────────────────────────────────────────────┘
```

The right aside is **not** always present — it appears when a zone/app is selected or when the user explicitly opens it. This keeps the canvas primary.

### 2.2 Region ownership rules
- The **shell** computes every region rect once per frame. No child guesses its siblings' sizes.
- **Only Main scrolls**; chrome (top bar, rails, status bar) stays pinned.
- Left rail and right aside have **independent internal scroll** (already true via `ScrollArea`).
- Footer height comes from a single token/config value; canvas gets `remaining = main_height - toolbar - footer`.
- Remove magic numbers: `stage_lower_h`, `available_height()-56`, `48.0`, fixed `max_height` values where they compete.
- **Library accessibility:** the left rail can be toggled/collapsed; when collapsed, a slim icon strip or a "Apps" toggle in the toolbar reopens it. The library is never more than one click away.

### 2.3 Responsive behavior
Use shadcn-egui's `layout::window_at_least(ctx, px)` and `layout::container_at_least(ui, px)` (both exist in 0.8, currently unused):
- **≥ 1100px:** full shell (left rail + main + right aside).
- **800–1100px:** right aside collapses to a toggle button; canvas expands.
- **< 800px:** left rail collapses to icon-only or a drawer; canvas takes most space.
- Set `ViewportBuilder::with_min_inner_size([900, 600])` in `main.rs` (currently missing).
- Panel collapse: `egui::Panel` has no `collapsible` method and shadcn has no `show_collapsible`; use `egui::CollapsingHeader` or conditionally skip `Panel::left/right` based on a toggle in `Editor` state.

---

## 3. Navigation model

| State | Location | Purpose |
|---|---|---|
| `view: View` (Canvas/Hotkeys/Settings) | `state.rs:112-117` | Main workspace |
| `rail_tab: RailTab` (Apps/Profiles) | `state.rs:105-110` | Left rail |
| `inspector_tab: InspectorTab` (Process/Browser/Automation) | `state.rs:97-103` | Right aside |
| `picker_tab: PickerTab` | `state.rs:210-215` | Picker dialog |

Navigation is coherent; no duplicated view state. Keep it. The redesign is about presentation, not routing.

**Change:** the right inspector should be **contextual** — shown only when a zone/app is selected, or collapsible — so it does not permanently compete with the canvas. The console moves to a collapsible section or its own tab.

---

## 4. Screen-level layouts (wireframes)

### 4.1 Home / Overview (new view)
```
┌──────────────────────────────────────────────────────────────┐
│ ZoneFlow   Active: Website ▾   [4K ▾]              Engine ●  │
├──────────────────────────────────────────────────────────────┤
│  ACTIVE LAYOUT                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ Website — [APPLIED]      Work area: 3840×2160         │  │
│  │ 3 zones · 2 apps assigned                             │  │
│  │ [Edit] [Apply] [Disable layout]                       │  │
│  └────────────────────────────────────────────────────────┘  │
│  PROFILES                                                    │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ Website   ✓ APPLIED   [Apply] [Edit] [Duplicate] [Del]│  │
│  │ Coding    — saved     [Apply] [Edit] [Duplicate]...   │  │
│  │ Trading   — saved     [Apply] [Edit] [Duplicate]...   │  │
│  └────────────────────────────────────────────────────────┘  │
│  [+ New profile]                                             │
├──────────────────────────────────────────────────────────────┤
│ Engine ready · 17 hotkeys · v0.1.0                           │
└──────────────────────────────────────────────────────────────┘
```
**State badges must be visually distinct:** `APPLIED` (green), `SAVED` (neutral), `UNSAVED ●` (amber), `DISABLED` (red/grey). The active profile is always shown in Home and the tray.

### 4.2 Layout Editor (primary workspace)
```
┌──────────────────────────────────────────────────────────────┐
│ ZoneFlow   Editing: Website (unsaved ●)   [4K ▾]             │
├────────────┬─────────────────────────────────┬───────────────┤
│ APPS       │ [Layout][Draw][Undo][Redo]      │ INSPECTOR     │
│ [+ Add]    │ [Split][Merge][Remove] Gap ──  │ (when zone    │
│ [Search]   ├─────────────────────────────────┤  selected)    │
│ 7-Zip      │                                 │ Label         │
│ AyuGram    │      ┌──────────┬────────┐      │ Exe           │
│ Chrome     │      │ Zone 1   │ Zone 2 │      │ Path          │
│ ...        │      │ (app)    │ (app)  │      │ Launch        │
│            │      ├──────────┤────────┤      │ URL / Title   │
│ PROFILES   │      │ Zone 3   │ Zone 4 │      │ Macro         │
│ Website    │      └──────────┴────────┘      │ [Console ▾]   │
│ Coding     │                                 │               │
│ [Tools]    ├─────────────────────────────────┤               │
│ C H S R ?  │ [Save] [Revert]        [Apply]  │               │
│            │ APPS IN THIS WORKSPACE (scroll) │               │
├────────────┴─────────────────────────────────┴───────────────┤
│ Ready · 3 zones · 2 assigned · Ctrl+drag to snap             │
└──────────────────────────────────────────────────────────────┘
```
Key changes from current: **Save** and **Apply** are separate buttons; inspector is contextual; console is collapsible; status bar at bottom.

### 4.3 Wizard / picker dialogs
- Wizard: larger, responsive width derived from `ctx.viewport_rect().width()`, live preview, readable template buttons.
- Picker: list fills available height; no empty void; tabs Installed / Running / Browse.

---

## 5. Component organization

- Keep the existing 13 modules; add `shell.rs` and `home.rs` (Home view) and `status_bar.rs`.
- Move `run_sequence` simulation out of the Launch button path (see product workflow §7).
- Split `mod.rs` responsibilities gradually: system actions, zone mutations, execution simulation into dedicated modules — only if it reduces the 1,280-line hub; avoid speculative abstractions.
- Use shadcn components consistently; replace remaining native `ui.label` / `egui::Label` in inspector/dialogs/action bar with shadcn `label` where a component exists.
- Keep the custom canvas painter (no shadcn equivalent); move its font sizes into `theme.rs`.

---

## 6. State management

- **Draft vs saved vs active** must be explicit in `Editor` state:
  - `draft: AppConfig` (editing)
  - `last_saved_json: String` (baseline for dirty)
  - `active_profile: String` (applied to Windows) — currently conflated with `cfg.active_profile`
  - `layout_enabled: bool` (new disable concept)
- **Profile-specific assignments are already in the data model** (see product workflow §1a): `Profile.zones: Vec<Zone>` (`config.rs:150-156`), each `Zone` owns exe/path/rules (`config.rs:84-109`), `attach_app` writes only to `active_mut().zones` (`mod.rs:585-596`). The global library is runtime enumeration (`rail.rs:485-569`), not persisted assignments. **No data-model rewrite is needed** — only UI clarity and the workflow rules in product workflow §2a/§3a.
- **Dirty check:** cache a dirty flag updated on mutation instead of serializing the whole config twice per frame (`dirty()` at `mod.rs:272-274`, called from `top_bar.rs:17` and `action_bar.rs:33`).
- **Config consistency:** hotkey actions currently call `config::load_config()` from disk (`mod.rs:356-385`) while auto-launch/display polling use `self.draft` (`mod.rs:1118,1200`). Unify on one source (the in-memory config) and reload from disk only on explicit Revert.
- **LOADED_SETTINGS** (`config.rs:252`) is frozen at first load; config-free helpers (`resolutions()`, `app_list_cap`) never see runtime edits. Replace with passing `&Settings` explicitly or refreshing the snapshot on save/revert.
- **History:** full-config JSON snapshots are heavy but acceptable for a small app; keep.
- **Visual state mapping:** the four states (saved / unsaved / applied / disabled) must each have a distinct badge in TopBar, Home, and StatusBar — never implied by position alone.

---

## 7. GUI / backend interaction

- GUI calls backend through a thin set of functions (`engine::*`, `system::*`, `autostart::*`). No trait/service layer needed for a small app — but make the **data flow** consistent (one config source).
- Long operations (auto-launch) already use a worker thread + mpsc; keep.
- Execution feedback: console lines from real operations, not just simulation. `run_sequence` must either perform real execution or be renamed.
- Error reporting: surface `System::failures` (hotkey conflicts) and engine errors in the console/status bar.

---

## 8. Design system

- Typography: define a scale in `theme.rs` (currently canvas fonts 9–12.5pt hardcoded); use shadcn's text roles for widgets.
- Spacing: `tokens.rs` already centralizes spacing; extend to cover all regions; remove remaining magic numbers.
- Color: shadcn Zinc+Amber theme is applied once in `main.rs`; `CanvasTheme` reads its palette. Keep. Zone tints from config.
- Visual hierarchy: canvas is the primary surface; secondary config is contextual/collapsible; status bar is always visible.

---

## 9. Feasibility corrections to earlier plan

- ❌ `StripBuilder` — does not exist in egui 0.37. Use Panels + rect math.
- ❌ shadcn `show_collapsible` — does not exist. Use `CollapsingHeader` or conditional panels.
- ✅ `window_at_least` / `container_at_least` — exist in shadcn 0.8, currently unused.
- ✅ `ViewportBuilder::with_min_inner_size` — exists in egui 0.37.
- App library cleanup is largely already implemented in `rail.rs:37-349`; only polish remains (WS-B/WS-C).
