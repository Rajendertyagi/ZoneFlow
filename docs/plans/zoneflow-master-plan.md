# ZoneFlow Studio — Master Plan

Last updated: 2026-11-21
Status: historical master plan. **The UI-stack decision in §9 (egui + glow LOCKED) was superseded on 2026-11-21** by the authoritative `zoneflow-rust-axum-svelte-architecture.md` (headless Rust + Axum + on-demand Svelte editor). Data model (§1), configurability (§2) and D1–D6 remain in force; §6 phases, §9, and the **threading paragraphs in §10.1 / §11.2** are historical — thread ownership is now defined in the new architecture plan §3.1 (main thread **is** the native thread and owns tray/hotkeys/overlay + message pump; a command worker and a Tokio `current_thread` runtime run on two other threads). **IMPLEMENTATION Phases 1–7 of the egui app COMPLETE and verified** (fmt clean, 43 tests pass, clippy clean, release build ~14.21 MiB). AHK detection removed per user.

> **Where this file fits (read first):** This is the historical master plan. The **authoritative architecture and migration plan is `zoneflow-rust-axum-svelte-architecture.md`**. For product behavior, `zoneflow-product-workflow.md` (D1–D6, frozen), `zoneflow-feature-preservation.md`, and `zoneflow-implementation-roadmap.md` remain authoritative; `zoneflow-gui-architecture.md` is historical for the web UI. All six product decisions **D1–D6 are APPROVED and frozen**. Multi-monitor is explicitly OUT OF SCOPE. **No implementation is authorized** by any planning document; implementation requires explicit user approval. Older sections below (§0–§15) are retained as background and may contain superseded wording (e.g. "decision pending", Tauri/egui-era references, ~10 MB size claims) — where they conflict with the authoritative plan or the product documents, those win.

**NEW ARCHITECTURE (2026-11-21):** The active implementation direction is now `docs/plans/zoneflow-rust-axum-svelte-architecture.md` — a headless Rust + Axum background process with an on-demand Svelte editor served from embedded assets. It supersedes §9's egui stack decision and the §6 phase list. The four documents below remain authoritative for product behavior (D1–D6, features, workflow); the new plan is authoritative for architecture and migration phases. `slint-gui-migration-plan.md` is retained as historical research only.

**CURRENT DIRECTION (historical, 2026-11-21):** Four independent audits + product research produced a revised planning set in the repository at `docs/plans/` (pushed to GitHub `Rajendertyagi/ZoneFlow`, single branch `main`; commits `3e02a25`, `504b3aa`, `7ab9a36`, `023a941`, `d03d63f`):
- `zoneflow-product-workflow.md` — product definition, user journeys, **profile-specific assignments confirmed already in data model**, exact workflow rules, **D1–D6 frozen**, Disable behavior.
- `zoneflow-gui-architecture.md` — canvas-primary shell, contextual inspector, no forced 3-column, four independent state dimensions, Home view, tray design.
- `zoneflow-feature-preservation.md` — 20-feature inventory, preservation matrix, 12 confirmed bugs (B1–B12), same-app limitation.
- `zoneflow-implementation-roadmap.md` — 4 coordinated workstreams (backend correctness, GUI shell, profile/app workflow, integration testing) + checkpoints + performance baseline strategy.
- `zoneflow-master-plan.md` — this file (historical master plan, moved into the repo from OpenCode's plan directory).

**Key correction:** earlier "Phases 1–7 complete" referred to the *implementation* work; the four `docs/plans/` documents are the **new source of truth for the next iteration** (product/GUI redesign + bug fixes). Multi-monitor explicitly OUT OF SCOPE. All six product decisions (D1 Apply modes, D2 unsaved changes, D3 assignment replacement, D4 Disable scope, D5 Launch/Preview, D6 rail/Home/tray switching) are **APPROVED** — no decisions pending. No implementation authorized yet.

**Audit disclosure:** commit `3e02a25` was not documentation-only — it also modified six application source files (`src/editor/canvas.rs`, `inspector.rs`, `mod.rs`, `rail.rs`, `theme.rs`, `toolbar.rs`) as an authorized bug-fix bundle. The two later commits (`504b3aa`, `7ab9a36`) are documentation-only. **Repository branches resolved (2026-11-21):** consolidated to a single `main` branch locally and remotely — empty legacy `main` deleted, `master` renamed to `main` (history preserved), GitHub default set to `main`. No history rewrite or force-push.

---

## 0. Read this first — why this plan was rewritten

This project has been rebuilt several times. The same mistakes recurred because the plan was vague about the **domain model** and let hardcoded data creep in. This version pins those down explicitly. Do not start work without reading §1 and §2.

### Mistakes already made (DO NOT REPEAT)
1. **Hardcoded apps in default profiles** — chrome.exe, Slack.exe, Obsidian.exe, URLs like developer.mozilla.org were baked into defaults. WRONG. Everything starts empty.
2. **Treated "apps" as a separate list** with an "Add App" flow disconnected from zones. WRONG — a zone IS an app binding (see §1).
3. **Hand-rolled the UI repeatedly** and it still looked worse than the reference. The reference is HTML/CSS; egui cannot match it exactly.
4. **Guessed APIs** instead of reading docs/source, causing many compile-fix cycles. ALWAYS read the actual API before writing code.
5. **Added CentralPanel before side panels** — egui requires CentralPanel LAST.
6. **Left the AHK script running** — it held the global hotkeys, so registration "failed". Not a code bug.

---

## 1. Domain model (LOCKED — this is the thing to get right)

Based on the user's ZoneFlow HTML reference and confirmed by research into FancyZones / PlasmaZones / PowerToys Workspaces.

### The relationship
- **Profile** = a named workspace with a hotkey. Contains zones.
- **Zone** = a rectangular screen region stored as **percentages** (x, y, w, h).
- **App** = bound to a zone, **one app per zone**. There is NO separate apps list. A zone *is* an app's place on screen.

### Fields on a Zone (from the reference)
| Group | Fields |
|---|---|
| Identity | label, exe |
| Launch | launch mode (launch if closed / attach if running) |
| Browser | url, title match, tab behavior (focus or Ctrl+T / reuse blank / ignore) |
| Macro | enabled, trigger title, keys, delay |
| Geometry | x, y, w, h (%) |

### Runtime behavior
- **Run Workspace** = for each zone in order: find or launch the app → move its window to the zone's pixel rect → apply URL/tab rule → fire macro if triggered.
- **Presets** (2 Columns / Priority / 3 Columns / 4 Grid) = geometry templates that rearrange zones while preserving each zone's app binding and rules.

### Two models considered (research finding)
- **A. Layout-first (FancyZones/PlasmaZones):** empty zones, fill by dragging windows or window rules. Layout has no app bindings.
- **B. Profile-first (PowerToys Workspaces / user's HTML):** each zone carries its app + rules; Run Workspace launches them all.

**DECISION: Model B** — the user's reference. But done correctly: empty by default, apps added by the user.

---

## 2. No hardcoding / maximum configurability (LOCKED)

This is a hard rule for the whole project, not just defaults. **Anything the user might reasonably want to change must be data, not code.**

### 2.1 Empty by default
- New profiles start with **ZERO zones**.
- No baked-in apps, URLs, titles, or labels anywhere in code or default config.
- Only generic **geometry templates** (Duo / Debug / 3 Columns / Focus / Empty) with empty zones the user fills.
- The default config file, if created, must contain either no profiles or profiles with empty zone lists.
- Any app data must come from the user via Browse (§3) or system enumeration — never a literal in source.

### 2.2 Everything configurable (data-driven)
The following must live in a single versioned config/schema file and be editable from the UI where the mockup exposes them:

| Area | Configurable values |
|---|---|
| Hotkeys | Every binding (the AHK defaults in §2c are **defaults only**, stored in config, editable in the Hotkeys view); both namespaces (§2c) |
| Appearance | All design tokens: colors, zone tint palette (default 6 colors — current code has 12, §12), spacing, radii, font stacks (§2d) |
| Layout | Gap default, grid columns/rows for Draw mode (default 12×8), snap points (default 25/50/75), min zone size % (default 15), resize step % (default 1), boundary tolerance % (default 1.5) |
| Editor | Undo stack depth (default 60), arrow-nudge step % (default 1, Shift 5) |
| Resolution | Preset list (4K / QHD / FHD) and custom target width/height/work-area |
| Behaviour | Close-to-tray, remember-choice, autostart, drag overlay, snap-on-launch (Settings view) |
| App sources | Start Menu paths, registry Uninstall keys (HKLM + HKCU), file-dialog filters |
| Labels/copy | All user-facing strings in one place (i18n-ready), no inline literals in UI code |
| Engine | DWM snap retries (default 2), display-change debounce ms (default 200), overlay opacity (default 80% inactive / 160% active), macro delay, min window size px (default 150×150) |

Every number above is a **default stored in config**, never a literal in code. §2d/§5/§6 prose states defaults only.

### 2.3 Config file rules
- **Location decision (C19):** user accepted "json with exe is ok", but if installed under `C:\Program Files` a non-admin cannot write there. Decide installer scope: either per-user install (writable next to exe) OR `%APPDATA%\ZoneFlow\config.json` with one-time migration from the exe directory. Record the choice before Phase 1.
- Versioned schema (`schema_version` field) with a documented migration path. Treat the current on-disk `ScreenLayout.json` as **v0**; migrate user profiles/zones (C23). "Empty by default" applies to new installs only — existing configs must preserve user data.
- Deserialize with defaults for missing fields (`#[serde(default)]`) so older configs keep working.
- Atomic writes: on Windows `std::fs::rename` fails if the destination exists — use `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING)` or remove-then-rename with a documented crash window (C20). Test the destination-exists case.
- No magic numbers in code — every tunable is a named constant sourced from config.

---

## 2a. Full workflow (LOCKED — from studying the "Layout" app, getlayout.app)

The user's desired workflow matches the **Layout** app almost exactly. Implement this flow:

1. **Create and name a profile** — "New profile" → name → starts EMPTY (template: Empty).
2. **Pick a layout template** — Duo / Editor+preview / Debug / Focus / Three columns / Stacked / Free. Templates are starting points; zones can then be split/removed/renamed/resized.
3. **Assign apps to zones** — the centerpiece:
   - Each empty zone shows **"Choose an app"** — click it.
   - An **app picker** lists: installed apps, running apps, apps used in other profiles, + "Other…" for a custom path.
   - Select → app assigned to that zone.
   - An **Apps list** below the preview shows every app with its zone.
   - **Drag an app row onto another zone** to move it.
   - Right-click a zone → Add App… / Rename… / Split Right / Split Down / Remove Zone.
4. **Launch the profile** — open apps not running, place each window in its zone, hide editor.
5. **Switch profiles** — keyboard shortcuts.
6. **Adjust zones** — drag dividers (1% steps, 10% min), Equalize, Reapply.

Key insight: the **app picker + attach flow** is the missing centerpiece — it's what makes this a real "add app → assign to zone" app rather than a config editor.

### App picker implementation
- Enumerate installed apps: scan Start Menu shortcuts (`%AppData%\Microsoft\Windows\Start Menu\Programs` and ProgramData equivalent) + registry Uninstall keys.
- Enumerate running apps: `EnumWindows` + process path (engine already has this).
- "Other…" → `rfd` file picker for `.exe`.
- Store: label, exe, full path, launch mode, url, title match, tab behavior, macro fields.

---

## 2b. Canvas interactions (LOCKED — from re-reading the HTML)

The HTML has a proper **shared-boundary mouse resize engine** that our current app lacks. Port it exactly:

### Shared-boundary resize (mouse)
- `findSharedBoundaries(zones)` — find where zones touch (tolerance default 1.5%, configurable §2.2):
  - **Vertical boundary** at x = bx where some zones' right edge (`x+w`) ≈ bx and others' left edge (`x`) ≈ bx. Span = min/max y of all touching zones.
  - **Horizontal boundary** at y = by where some zones' bottom (`y+h`) ≈ by and others' top (`y`) ≈ by. Span = min/max x of all touching zones.
- Render a **resize handle** on each boundary (`.resize-handle-v` / `.resize-handle-h`): a thin pill that highlights on hover/drag.
- On drag (`startBoundaryDrag` → `onMove`):
  - Compute delta in % from mouse movement along the axis.
  - Clamp new position: `minAllowed = max(before zones' x+minSize)`, `maxAllowed = min(after zones' (x+w)-minSize)` (minSize default 15%, configurable §2.2).
  - Resize **all zones on both sides**: before zones get `w = newPos - x`; after zones get `x = newPos`, `w = rightEdge - newPos` (analogous for horizontal with y/h).
  - **Live HUD** pill: `Split X: 50% (1920px | 1920px)` / `Split Y: ...`.
  - On release: log "Updated zone dimensions via interactive resize."
- This is different from the current app, which only moves whole zones — divider resize must be added.

### Context menu on the grid (NEW — not in HTML, user-requested)
- Right-click a zone → context menu with actions:
  - Split left/right
  - Split top/bottom
  - Delete / merge zone
  - Add app to this zone (browse)
  - Duplicate zone (optional)
- Right-click empty canvas → "Add app" / "Reset layout" (optional).
- In Tauri: implement the context menu in the HTML frontend (§2d) — hit-test the zone under the cursor in JS, show a DOM menu clamped to the viewport. (If egui fallback: `response.context_menu(|ui| ...)`.)

### Zone move (existing, keep)
- Dragging a zone body moves it (clamped to 0–100%). Keep alongside divider resize.

---

## 2c. Hotkeys + feature parity with the original AHK script (LOCKED)

The user wants the new app to attach the **same keyboard shortcuts** as the existing `E:\Apps\Hotkeys\ScreenLayout.ahk`. Exact list from that script:

| Hotkey | Action | AHK source |
|---|---|---|
| `Ctrl+Shift+1` … `Ctrl+Shift+9` | Launch/focus app by index | `Hotkey("^+" . idx, ProcessApp.Bind(idx))` |
| `Ctrl+Shift+0` | Re-tile all apps | `Hotkey("^+0", ReTileOpenApps)` |
| `Ctrl+Alt+Left` | Move active window to previous zone | `CycleZone(-1, 0)` |
| `Ctrl+Alt+Right` | Move active window to next zone | `CycleZone(+1, 0)` |
| `Ctrl+Alt+Up` | Move active window up a row | `CycleZone(0, -1)` |
| `Ctrl+Alt+Down` | Move active window down a row | `CycleZone(0, +1)` |
| `Win+Up` | Maximize active window | `MaximizeActive()` |
| `Win+Down` | Restore active window to its zone | `RestoreToZone("A")` |
| `Ctrl+Alt+R` | Reload script/config | `Reload()` |
| `Ctrl + drag` window title bar | Drag-to-snap with zone overlays | `~^LButton` → `DragToSnap` |

### Other AHK features to preserve
- **Auto-launch and snap all apps on startup** (`AutoLaunchAndSnapAll`)
- **Display/work-area change listener** — `WM_DISPLAYCHANGE` (0x007E) and `WM_SETTINGCHANGE` (0x001A, wParam 0x002F = SPI_SETWORKAREA) → recalculate zones and re-tile
- **DWM zero-gap snapping** — `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE=33, DONOTROUND=1)` + `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS=9)` for invisible borders; apply snap twice
- **Window matching** by exe name or full path (`matchType: exe|path`)
- **Drag overlays** — translucent always-on-top zone windows with numbers, highlight active zone (80% inactive / 160% active opacity)
- **Tray menu** — Settings / Re-tile / Reload / Exit

### Count and namespaces (IMPORTANT — corrected)
- **17 keyboard bindings** + **1 mouse gesture** = 18 behaviours, NOT "10 hotkeys". Breakdown: Ctrl+Shift+1…9 (9) + Ctrl+Shift+0 (1) + Ctrl+Alt+arrows (4) + Win+Up/Down (2) + Ctrl+Alt+R (1) = **17 keyboard**; Ctrl+drag title bar = **1 mouse gesture**.
- **Ctrl+drag is a low-level mouse gesture**, detected via polling `GetAsyncKeyState(VK_CONTROL|VK_LBUTTON)` in the egui frame loop (already implemented in `engine.rs:328–334`, §5) — it is NOT a global shortcut. Count and verify it separately from the 17 keyboard bindings.
- **Two hotkey namespaces:** (a) global AHK-parity bindings above, stored in a `hotkeys` map in config; (b) optional per-profile switch bindings from `Profile.hotkey`. The current `Profile.hotkey` field is **dead** (never consumed) — wire it to profile switching or remove it in Phase 1.

### Notes
- `Win+Up`/`Win+Down` may fail to register because Windows reserves them — acceptable; document explicitly if so.
- The AHK script must be stopped before running the new app (it holds the same hotkeys). Add a **startup detection** that warns/refuses hotkey registration if the AHK process is running (C25).

---

## 2d. UI/UX specification (LOCKED — finalized in `design-v3.html`)

The standalone HTML mockup `E:\Apps\Hotkeys\ScreenLayoutRust\design-v3.html` is the **visual and interaction target**. It is a pure front-end demo (localStorage persistence, no backend). Port this exact behaviour to the real app regardless of stack (§9).

### Layout — three panes
- **Left rail**: app library (searchable, draggable), Profiles list with New / Duplicate / Rename / Delete, and a **Tools dock** at the bottom (Canvas / Hotkeys / Settings / Reset Demo / Help icon buttons).
- **Center**: workspace bar, canvas stage, action bar (Revert / Save & Apply / Launch), and a fixed-height scrollable "Apps in this workspace" strip.
- **Right**: Zone & Automation inspector — live-edited fields, console log. Hidden in Hotkeys/Settings views.

### Visual language
- Deep slate + warm amber. **Hairline borders** via `inset 0 0 0 1px` shadows, **shadow-based elevation** (3-step scale), no heavy outlines.
- **Zone color tints** (default 6-color palette, configurable §2.2 — current egui code has 12, §12) differentiate zones: left accent bar, number badge, exe text, pixel readout.
- Inspector section headers carry subtle accent bars. All colors/spacing/radii are design tokens (§2.2).

### Canvas interactions
- Shared-boundary divider resize with live HUD and **snap guides at 25 / 50 / 75%** (§2b).
- Drag app from library → zone to attach; drag zone → zone to swap.
- **Draw mode**: toggle overlays a configurable grid (default 12×8, §2.2); drag to paint grid-aligned zones; snap toggle.
- **Pick-target merge**: click Merge → source zone highlighted, other zones show "MERGE HERE" → click target → merge into bounding rect (target's binding survives); Esc / empty canvas / button toggles cancel.
- Right-click zone → context menu (Add app, Rename, Split 50/50, Split 1/3·2/3, Split 2/3·1/3, free split right/down, Merge with…, Remove). Menu clamped to viewport.

### Layout wizard (primary creation flow)
`＋ Layout` button opens a modal asking "What do you want to do?" with a **live preview**:
- Use a template (Duo / Debug / 3 Columns / Focus / Empty)
- Custom grid (columns × rows → equal zones)
- Single zone (half/third/quarter/center presets)
- Draw on grid (closes modal, enables Draw mode)

### Editing & history
- Inspector edits sync live to canvas + apps strip; one undo step per focus session.
- **Undo/Redo** (stack depth default 60, configurable §2.2): `Ctrl+Z`, `Ctrl+Shift+Z` / `Ctrl+Y`, plus toolbar buttons.
- Editor shortcuts: `F2` rename, `Delete`/`Backspace` remove, `Esc` deselect/cancel, arrows nudge 1% (`Shift` 5%). Suppressed while typing or in a dialog.
- **Unsaved changes** badge; Revert disabled and Save shows "Saved" when clean, "Save & Apply" when dirty.
- Resolution preset selector (4K / QHD / FHD) updates pixel readouts and drag HUD. Gap slider adjusts spacing live.

### App picker
Modal with Installed / Running now / Browse… tabs. Browse is a clean path input + simulated file pick; filename becomes label. (Real app uses native `rfd` picker — §3.)

### Execution simulation
Launch / Test zone append realistic step-by-step lines to the console (checking window → applying bounds → URL → macro → done).

### Persistence
Mockup uses `localStorage`; real app uses the versioned config file (§2.3). Reset Demo restores empty starter profiles (Website / Coding / Trading) — **with empty zones** in the real app.

---

## 3. Browse + Add App flow (LOCKED)

Since a zone is an app binding, "Add App" = **create a new zone with that app**.

1. **Add App** button in the workspace rail (next to "+ New Profile") and/or canvas toolbar.
2. Dialog:
   - **Browse** button → native file picker (`.exe`) → auto-fill label (filename without extension), exe (filename), path (full path, stored for reference).
   - Editable: label, launch mode, url, title match, tab behavior, macro fields.
   - **Placement**: split the currently selected zone left/right or top/bottom (default: split selected in half) so the new zone gets immediate space.
3. On Save: new zone created with app binding + geometry from split; becomes selected.
4. Draft workflow preserved — nothing applied until **Save & Apply**.

Dependency: `rfd` crate (standard lightweight native file dialog).

---

## 4. Current implementation state (what exists now)

Folder: `E:\Apps\Hotkeys\ScreenLayoutRust\`

| File | Role | Status |
|---|---|---|
| `Cargo.toml` | deps | eframe/egui 0.37 (glow), shadcn-egui 0.8, windows 0.62, tray-icon, global-hotkey, rfd (to add) |
| `src/config.rs` | config model | **NEEDS REWRITE** — currently has hardcoded profiles (§2) |
| `src/engine.rs` | Win32 engine | mostly OK — zone math, gap, window snap, URL/tab/macro, hotkeys, overlay |
| `src/editor.rs` | UI | **NEEDS REWORK** — add browse/add-app, empty defaults, UI quality open (§9) |
| `src/system.rs` | tray + hotkeys | OK |
| `src/overlay.rs` | drag-to-snap overlay | OK |
| `src/autostart.rs` | registry auto-start | OK |
| `design-v3.html` | UI/UX target mockup | DONE — pure front-end demo, all interactions in §2d implemented and browser-verified |

Original AHK files are untouched at `E:\Apps\Hotkeys\ScreenLayout.ahk` and `E:\Apps\Hotkeys\ScreenLayoutGUI\`.

The mockup is the **specification**, not the shipping UI. The real app must reproduce §2d behaviour; the stack used to do that is the open decision in §9.

---

## 5. Verified working (do not re-debug)
- Clean release build, no warnings, ~10 MB exe, ~49 MB private RAM (glow renderer; wgpu caused 323 MB — keep glow).
- Zone canvas with polished cards (number badge, label, exe, Tiled pill, URL badge, %/px footer), monitor bezel, taskbar strip.
- Inspector populated from selected zone; friendly enum labels ("Focus or Ctrl+T", "Launch if closed").
- Tray + hotkeys register cleanly **when the AHK script is not running**.
- Close dialog, auto-start, drag-to-snap overlay.

---

## 6. Phase-wise implementation plan (AUTHORITATIVE)

Basis: official API/docs fetched 2026-11-20 (§11) + full audit of the current Rust code (§12). Each phase has an **exit gate** — do not start the next phase until it passes. Every phase ends with `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and a release build clean.

### Phase 0 — Foundation (stack already DECIDED: egui + glow)
**Goal:** make the build reproducible, remove dead weight, lock the renderer. No feasibility spike needed — the overlay and Ctrl+drag already work in the current egui build (§5).
- [ ] Stack is **locked**: pure eframe/egui 0.37 + glow, no HTML/webview (§9). Do not revisit.
- [ ] Add `rust-toolchain.toml` pinning current stable (egui 0.37 needs recent stable).
- [ ] **Keep** `eframe`, `egui`, `shadcn-egui`, glow. **Remove only the dead `egui_tiles` dependency** (§12).
- [ ] **Verify Cargo.toml keeps `default-features = false, features = ["glow", ...]`** — never let wgpu become the renderer (wgpu ~323 MB vs glow ~49 MB, C30).
- [ ] Release profile: `opt-level=2`, `lto=true`, `strip=true` (current). Consider `panic="abort"` only after zero unwraps are confirmed (C28).
- [ ] CI skeleton: fmt, clippy, test, release build on every push.
**Exit gate:** clean release build with glow renderer confirmed; `egui_tiles` gone; no HTML/webview dependency anywhere.

### Phase 1 — Config foundation ✅ DONE (2026-11-20)
**Goal:** replace the hardcoded, unversioned config with a durable, data-driven one.
- [x] Versioned schema: `schema_version` (v1), `#[serde(default)]` on all structs, `#[serde(default = "zero")]` on `schema_version` so v0 configs are detectable. `migrate()` bumps v0 → v1. (No `#[serde(from/into)]` needed — additive migration via defaults is sufficient here.)
- [x] Empty by default: `default_config()` returns 3 starter profiles (Website/Coding/Trading) with **zero zones**, no hardcoded apps/URLs/titles/macros.
- [x] Tunables in a `Settings` struct: snap points, grid cols/rows, min zone %, resize step, boundary tolerance, undo depth, display debounce, overlay alphas, DWM retries, min window px, zone tints. `gap`/`close_action`/`auto_start` kept at top level for call-site compatibility.
- [x] `path` field added to `Zone`.
- [x] Atomic writes: temp file → remove dest → rename (pragmatic alternative to `MoveFileExW`; small documented crash window, config kept in memory). Unreadable configs renamed to `.json.bad` instead of silently wiped.
- [x] v0 migration preserves user profiles/zones; "empty by default" applies to new installs only.
- [x] Panic-free `active()`/`active_mut()` via a `#[serde(skip)]` fallback profile and `entry().or_insert_with()` self-healing — no `thiserror` needed, no panics.
- [x] Unit tests: 5 tests — empty defaults, round-trip, missing-field defaults, v0 migration, panic-free helpers. All pass.
- [ ] **Deferred to Phase 4 (UI rebuild):** `dirty()` disk re-read fix (C18) — needs an in-memory `last_saved_json` in the editor, which is being rewritten anyway.
- [ ] **Deferred:** hotkeys/labels/app-source paths into config — the Hotkeys/Settings UI that edits them is part of the Phase 4 UI rebuild.
**Exit gate MET:** no hardcoded user data remains; config survives missing fields and old versions; v0 config migrates; 5 tests pass; `cargo build --release` and `cargo clippy -- -D warnings` clean.

### Phase 2 — Backend engine hardening (stack-independent)
**Goal:** make the existing Win32 engine safe, testable, and reusable as a library.
- [ ] Extract **only the pure logic** into `engine-core` (no Win32): zone math, boundary detection, 25/50/75 snapping, percent↔pixel, DWM border-compensation arithmetic. Only ~4 of ~20 engine functions are pure (§12) — do not overstate coverage.
- [ ] Keep all window enumeration/movement/input in `engine-win32` behind `#[cfg(windows)]`. Enable features derived from **actual imports** (§11.1): `Win32_Foundation`, `Win32_System_Registry`, **`Win32_System_Threading`** (process path lookup — required, C2), `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_Graphics_Dwm`. Add `Win32_UI_Shell`/`Win32_Graphics_Gdi` only if actually needed.
- [ ] Document all 17 `unsafe` blocks with `// SAFETY:` comments (audit §12.5 — currently zero).
- [ ] Replace 19 `let _ =` swallowed results with `tracing::warn!` or typed errors (§11.4); remove 8 `unwrap()` + 4 `expect()` from library code.
- [ ] Add `tracing` + `tracing-subscriber` with `env-filter`, file writer, no ANSI (§11.4); remove `println!` debugging.
- [ ] Unit tests for zone math, `findSharedBoundaries`, 25/50/75 snapping, DWM border compensation math.
- [ ] Mock Win32 behind a trait **only at the `engine-win32` seam**; the trait must not expose `HWND` across threads (`!Send`/`!Sync`, §11.1).
- [ ] **Threading (C4 — HISTORICAL, egui era):** pure window-manipulation calls may run on a worker thread with typed channels, but **global-hotkey manager, tray icon, and all message-loop-dependent code must be created and owned on the egui/eframe main thread** (which owns the winit/Win32 message loop). The current `system.rs` already does this correctly — preserve it. **Superseded:** in the new architecture the main thread **is** the native thread and owns these managers, the overlay and the message pump; see `zoneflow-rust-axum-svelte-architecture.md` §3.1.
**Exit gate:** engine-core tests pass on non-Windows; zero undocumented unsafe; zero swallowed errors in library code; hotkey/tray managers confirmed on the owning native thread (new architecture).

### Phase 3 — Editor UI rebuild, part 1: shell & canvas (egui)
**Goal:** rebuild the editor shell and canvas in egui from the `design-v3.html` spec (§2d). Use egui 0.37 APIs (§7, §11.6).
- [ ] Define design tokens in Rust: a `Theme` struct with colors (deep slate + warm amber), zone tint palette (default 6), spacing, radii, font sizes — all sourced from config (§2.2). Apply via `Context::set_visuals` / `Visuals` and per-widget `Color32`.
- [ ] Three-pane shell: `Panel::left` (app library + profiles + tools dock), `Panel::right` (inspector), `CentralPanel` **added LAST** (§7). Panels use hairline strokes (`Stroke` + `round_to_pixel_center`) and `Frame::shadow()` / `egui::Shadow` for elevation (C8 — egui supports shadows).
- [ ] Canvas: custom-painted monitor area with `Painter` — zone rects with tints, number badges, labels, exe, Tiled pill, %/px footer, bezel, taskbar strip. Hit-testing via `ui.interact(rect, id, sense)` (§11.6).
- [ ] Shared-boundary resize with live HUD and 25/50/75 snap guides (§2b) — port the working logic from the current `editor.rs`.
- [ ] Drag app → zone, drag zone → zone swap, pick-target merge, context menu via `Response::context_menu`, drop-target highlights.
- [ ] Draw mode: grid overlay + drag-to-paint grid-aligned zones (§2d).
**Exit gate:** canvas renders all §2d visual elements; divider resize, snap guides, drag/drop, merge, context menu and draw mode all work in egui.

### Phase 4 — Editor UI rebuild, part 2: wizard, inspector, history (egui)
**Goal:** finish the egui UI to full §2d parity.
- [ ] Layout wizard modal via `egui::containers::modal::Modal` (§11.6): template / custom grid / single zone / draw, with a live preview painted via `Painter`.
- [ ] Inspector: live-editing fields via `TextEdit`, `ComboBox`, `Checkbox`, `Slider` (shadcn-egui or native egui) — all §2d fields sync live to the selected zone and apps strip.
- [ ] Undo/redo command stack (depth from config §2.2): `Ctrl+Z`/`Ctrl+Shift+Z`/`Ctrl+Y` via `InputState::consume_shortcut` (§11.6), plus toolbar buttons. One undo step per focus session.
- [ ] Editor shortcuts: F2 rename, Delete remove, Esc deselect/cancel, arrows nudge 1%/Shift 5% — suppressed while typing or in a dialog.
- [ ] Dirty state: in-memory `last_saved_json` snapshot (C18), unsaved badge, Revert/Save & Apply reflecting state.
- [ ] Resolution presets (4K/QHD/FHD) and gap slider updating pixel readouts and drag HUD live.
- [ ] App picker modal: Installed / Running / Browse tabs; Browse via `rfd` native file dialog (§11.4) — add `rfd` dependency.
- [ ] Execution simulation: step-by-step console lines on Launch / Test (§2d).
- [ ] Sidebar tools dock: Canvas / Hotkeys / Settings / Reset Demo / Help icon buttons; Hotkeys view bindings editable from config; Settings toggles wired to config + `autostart`.
**Exit gate:** every §2d interaction works in egui; UI matches the mockup's layout and visual language (small rendering differences accepted).

### Phase 5 — System integration (mostly already done — verify + wire config)
**Goal:** the Win32 systems code already exists (§5); wire it to config and verify. No new plugins needed.
- [ ] Tray: existing `system.rs` menu (Settings / Re-tile / Reload / Exit) — keep; ensure labels come from config (§2.2).
- [ ] Global hotkeys: existing `system.rs` registers 17 keyboard bindings — **load them from config** (§2c), not hardcoded. Ctrl+drag stays the existing mouse-gesture path (already works, §5).
- [ ] Autostart: existing `autostart.rs` — driven by Settings toggle + config value.
- [ ] File dialog: add `rfd` for the Browse tab (Phase 4); native `.exe` picker.
- [ ] Display-change listener: existing `engine.rs`/`system.rs` handles `WM_DISPLAYCHANGE`/`WM_SETTINGCHANGE` — debounce default 200 ms (configurable §2.2).
- [x] **AHK script detection — REMOVED per user (2026-11-20):** the Phase 5 agent added `ahk_script_running()` + a startup warning, but the user decided the app does NOT need to check for AHK. Remove that function and the warning from `system.rs` after the UI agent finishes. The app has no AHK dependency; hotkey registration simply fails gracefully if a binding is already grabbed (logged via `tracing`).
**Exit gate:** tray menu works; all 17 keyboard shortcuts fire with AHK script stopped (Win+Up/Down confirmed working or documented as reserved); Ctrl+drag works; autostart toggles; file picker opens; display change re-tiles.

### Phase 6 — Feature parity & polish
**Goal:** close the gap with the original AHK script and the mockup's execution simulation.
- [ ] Auto-launch and snap all apps on startup (AHK `AutoLaunchAndSnapAll`).
- [ ] DWM zero-gap snapping: `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE=33, DONOTROUND=1)` + `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS=9)`; apply snap twice (§2c, §11.1).
- [ ] Drag-to-snap overlay: existing `overlay.rs` already implements translucent always-on-top zone windows with numbers and 80%/160% opacity via egui `show_viewport_deferred` + `with_mouse_passthrough(true)` (§5). Keep it; make opacity configurable (§2.2). No new mechanism needed.
- [ ] Window matching by exe name or full path (`matchType: exe|path`) from config.
- [ ] Execution simulation: stream realistic step-by-step console lines from the backend during Launch / Test (§2d).
- [ ] Reset Demo restores empty starter profiles (Website / Coding / Trading) — **zero zones** in the real app.
**Exit gate:** full AHK feature parity; mockup execution simulation works against real backend.

### Phase 7 — Verification, docs & release
**Goal:** prove it works and make it maintainable.
- [ ] Full §8 checklist, including hotkeys with AHK script stopped.
- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, release build — all clean.
- [ ] Measure actual binary size and RAM; update README (currently claims ~6 MB but actual is ~10 MB — audit §12.10).
- [ ] README: install, config location, hotkeys, how to add apps, architecture note (§1) and where to add configurables (§2.2).
- [ ] CHANGELOG + semver; config `schema_version` tracks breaking changes.
- [ ] **Installer/signing (C24):** choose WiX/MSI vs NSIS for the single `.exe`; code signing matters for a startup-injected hotkey app (SmartScreen/AV). No WebView2 mode needed — there is no webview.
- [ ] **Re-verify after UI rebuild (C29):** overlay, close dialog, autostart and tray were verified in the current egui build (§5) — re-verify them after the UI rebuild and config rewrite; do not assume they carry over.
- [ ] CI artifacts attached to tagged releases.
**Exit gate:** every §8 box checked; docs match reality; release produced.

### Phase dependency order
```
Phase 0 (foundation) → Phase 1 (config) → Phase 2 (engine) → Phase 3 (UI shell/canvas) → Phase 4 (UI wizard/inspector/history) → Phase 5 (system wiring) → Phase 6 (parity) → Phase 7 (verify)
```
Phases 1 and 2 are independent and can run in parallel. Phases 3–4 are the egui UI rebuild from the `design-v3.html` spec. Phase 5 is mostly verification of already-working systems code.

---

## 7. API rules (learned the hard way)
- ALWAYS read the crate source in `C:\Users\RTPC\.cargo\registry\src\...` or docs.rs before using an unfamiliar API. No guessing.
- egui 0.37: `App::ui(&mut self, ui, frame)` (not `update`); `Panel::show(ui, ...)`; **CentralPanel added LAST**; `Id::unique` not `Id::new`.
- shadcn-egui 0.8: `Label::new(text).variant(...)`, `Button::new(text).variant(ButtonVariant)`, `Checkbox::new(&mut bool)` (1 arg), `Slider::new(&mut f32, RangeInclusive<f32>)`, `Card::new().show(ui, |ui| ...)`, `Input::new(&mut dyn TextBuffer)`.
- windows 0.62: `RegCreateKeyW` (not Ex), `RegSetValueExW(..., lpdata: Option<&[u8]>)`, `VIRTUAL_KEY(vk)` for wVk, `HWND.0.is_null()`.

---

## 8. Verification checklist (before claiming done)
- [ ] `cargo build --release` clean, no warnings; `cargo clippy -- -D warnings` clean; `cargo fmt --check` clean
- [ ] `cargo test` passes (zone math, config round-trip, migration)
- [ ] Default config has no hardcoded apps, URLs, titles, or labels
- [ ] Every value in §2.2 is in config and editable where the UI exposes it; no magic numbers in code
- [ ] Config has `schema_version`, survives missing fields, writes atomically, migrates old versions
- [ ] App picker lists installed + running apps + "Other…" native file picker
- [ ] Empty zone shows "Choose an app"; Apps list shows app → zone; drag app onto zone works
- [ ] Add App creates a zone with correct geometry and binding
- [ ] All 17 keyboard shortcuts work (§2c, loaded from config); Win+Up/Down confirmed working or documented as reserved; Ctrl+drag verified via mouse-hook path; auto-launch on startup; display-change re-tiles
- [ ] Empty profile shows add-first-app prompt / wizard
- [ ] Divider resize moves both sides with live HUD and 25/50/75 snap guides
- [ ] Draw mode paints grid-aligned zones; pick-target merge works with Esc cancel
- [ ] Layout wizard: all four options + live preview work
- [ ] Undo/Redo, F2/Delete/Esc/arrow shortcuts, dirty badge, resolution presets, gap all work
- [ ] Right-click a zone opens the full context menu (§2d), clamped to viewport
- [ ] Save & Apply persists and re-tiles; Reset Demo restores empty starter profiles
- [ ] Hotkeys work with AHK script stopped (no in-app AHK detection — removed per user)
- [ ] Existing v0 `ScreenLayout.json` migrates without data loss
- [ ] Overlay, close dialog, autostart and tray re-verified in the Tauri build (not just egui)
- [ ] UI decision (§9) resolved and applied; real UI matches `design-v3.html` (§2d)

---

## 9. UI stack — DECIDED: pure eframe/egui 0.37 + glow (2026-11-20)

**Decision: build the entire app in Rust with eframe/egui 0.37 and the glow renderer. No HTML, CSS, JS, or webview in the shipped application.** The `design-v3.html` mockups stay on disk as the **visual and interaction specification** (§2d) only — they are never loaded, bundled, or shipped.

### Why this stack (user's stated priorities: lowest RAM + fastest finish)
- **Lowest RAM:** ~49 MB private RAM measured with glow in this project; wgpu caused ~323 MB (rejected). WebView2 (Tauri) is a separate browser-engine process at 100+ MB. egui+glow is the clear winner.
- **Fastest finish:** the hard Win32 systems code already exists and works — `engine.rs` (window enumeration, snapping, DWM), `system.rs` (tray + 17 hotkeys), `overlay.rs` (drag-to-snap), `autostart.rs`. Tauri would discard all of it and rebuild through plugins + IPC, plus prove the overlay works in a webview. egui only needs a config rewrite and a UI rebuild.
- **No feasibility spike needed:** the overlay and Ctrl+drag already work in the current egui build (§5) — the Phase 0 Tauri spike (old C6) is obsolete.

### What this means for the plan
- **Keep:** `eframe`, `egui`, `shadcn-egui`, glow renderer. Remove only the dead `egui_tiles` dependency (§12).
- **Keep `default-features = false, features = ["glow", ...]`** in Cargo.toml — never let wgpu become the renderer (C30).
- **Rebuild the editor UI** in `editor.rs` from `design-v3.html` as the spec (§2d): three panes, hairline borders via `Stroke`, shadow elevation via `egui::Shadow`/`Frame::shadow()` (C8 — egui DOES support shadows), color tints via `Color32`, wizard modal via `egui::containers::modal::Modal`, context menus via `Response::context_menu`, drag/drop via `Response::dnd_*`, inline editing via `TextEdit`, undo/redo via a command stack.
- **Visual fidelity caveat:** egui cannot match browser text rasterization or the exact CSS cascade pixel-for-pixel. The rebuild targets the same layout, interactions and visual language, accepting small rendering differences. This is a careful rebuild from the approved design — NOT patching the old UI the user rejected.

### Rejected alternatives (kept for reference)
- **Tauri v2** (§11.3): exact visual fidelity but higher RAM and slower finish; rejected on the user's new priorities.
- **GPUI** (§14): pre-1.0, frozen crate, maintainer deprioritized non-Zed work, no first-party tray/hotkeys/overlay widgets, "read Zed source" docs; rejected.

**`design-v3.html` (§2d) remains the behavioural and visual target for the egui rebuild.**

---

## 10. Engineering standards — durable, modern, standard practice (LOCKED)

The solution must be maintainable long-term, not a one-off script. Apply these from the first line of code.

### 10.1 Rust backend
- **Edition 2024**, current stable toolchain; `rustfmt` and `clippy -- -D warnings` clean in CI.
- **Error handling**: `thiserror` for typed domain errors, `anyhow` only at binary boundaries. **Zero `unwrap`/`expect` in both library AND binary code** (C28 — with `panic="abort"` any missed one kills the app); no silent `let _ = ...` swallowing errors.
- **Logging**: `tracing` + `tracing-subscriber` with levels; no `println!` debugging in shipped code.
- **Config**: `serde` with versioned schema, `#[serde(default)]`, atomic write (`MoveFileExW` replace-existing), migration function tested against prior versions.
- **Concurrency (C4 — HISTORICAL, Tauri/egui era):** keep **pure window-manipulation calls** on a worker thread with typed channels. **Global-hotkey manager, tray icon, and all message-loop-dependent code must be created and owned on Tauri's main thread** (which runs the winit/Win32 message loop). Tauri commands run on the main thread by default — keep manager creation there; do not move it to the worker. **Superseded:** the new architecture uses the main thread as the native thread for these managers and the message pump; see `zoneflow-rust-axum-svelte-architecture.md` §3.1.
- **Tests**: unit tests for zone math / boundary detection / snapping / config migration; integration tests for config round-trip and a real v0 config file. Mock Win32 behind a trait at the `engine-win32` seam (must not expose `HWND` across threads). Add a hotkey-conflict test (C27).
- **Dependencies**: commit `Cargo.lock`; review new crates before adding; keep the dependency tree small (lightweight is a core requirement). **Keep `default-features=false, features=["glow",...]`** — wgpu caused ~323 MB RAM vs ~49 MB with glow (C30). Add `rfd` for the file dialog (Phase 4).
- **Unsafe**: isolate in small, documented modules with `// SAFETY:` comments; prefer `windows` crate safe wrappers.

### 10.2 UI layer (egui — §9)
- **Design tokens** (§2d) in one Rust `Theme` struct sourced from config — no hardcoded colors or sizes in widget code.
- **Structure**: split `editor.rs` into modules (canvas, inspector, wizard, dialogs, history) rather than one giant file; the mockup's single HTML file is a spec, not the shipping architecture.
- All state transitions go through one place so undo/redo and dirty state stay consistent (in-memory `last_saved_json`, C18).
- Use egui 0.37 APIs from §11.6 (Modal, context_menu, dnd_*, TextEdit, consume_shortcut, Shadow).

### 10.3 Cross-cutting
- **Semantic versioning** + CHANGELOG; config `schema_version` tracks breaking changes.
- **Reproducible builds**: pinned toolchain (`rust-toolchain.toml`), lockfiles committed, documented build steps in README.
- **CI**: format, lint, test, release build on every push; artifacts attached to tagged releases.
- **Docs**: README covers install, config location, hotkeys, and how to add apps; architecture note explains the domain model (§1) and where to add new configurables (§2.2).
- **No hardcoded user data** anywhere — enforced by code review against §2 and the §8 checklist.

---

## 11. Official API research (fetched 2026-11-20 — do not guess, verify against these)

### 11.1 windows 0.62
- docs.rs is a **stub**; real docs: `https://microsoft.github.io/windows-docs-rs/`.
- Cargo features — **derive from actual imports, not a fixed list** (C2): `Win32_Foundation`, `Win32_System_Registry`, **`Win32_System_Threading`** (required: `OpenProcess`/`QueryFullProcessImageNameW` for process path lookup), `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_Graphics_Dwm`. Add `Win32_UI_Shell`/`Win32_Graphics_Gdi` only if the app-picker/overlay actually needs them (current code does not use Gdi).
- `RegCreateKeyW` / `RegSetValueExW` / `RegGetValueW` return **`WIN32_ERROR`**, not `Result` — check with `.ok()?`. String args use `Param<PCWSTR>` (accept `&str`/`String`).
- `VIRTUAL_KEY` is a **newtype struct**, not an enum. `HWND` is `!Send`/`!Sync`.
- `DWMWA_WINDOW_CORNER_PREFERENCE = 33` (write via `DwmSetWindowAttribute`); `DWMWA_EXTENDED_FRAME_BOUNDS = 9` (read via `DwmGetWindowAttribute`). Corner preference needs Windows 11 Build 22000+.
- `WM_DISPLAYCHANGE = 126`, `WM_SETTINGCHANGE = 26`. `EnumWindows`/`EnumDisplayMonitors` take `unsafe extern "system"` callbacks; return `true` to continue.
- All calls are `unsafe` — every block needs `// SAFETY:`.

### 11.2 tray-icon 0.26 / global-hotkey 0.10
- `TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip(..).with_icon(..).build()`; menu module re-exports `muda 0.21`. Events: `TrayIconEvent::receiver()` **or** `set_event_handler(Some(..))` — the latter disables the former. Pick one.
- `GlobalHotKeyManager::new()?.register(HotKey)?`; parse via `"shift+alt+KeyQ".parse::<HotKey>()`; event `id` maps back via `hotkey.id()`. Windows: manager must be created on the event-loop thread. `register()` fails if combo is grabbed by another app.
- Both need an event loop on their thread — in the historical egui/Tauri design the main thread owned it; in the current architecture the main thread **is** that native thread (`zoneflow-rust-axum-svelte-architecture.md` §3.1).

### 11.3 Tauri v2 (EVALUATED, NOT CHOSEN — see §9)
- Latest stable `tauri 2.12.7`. Structure: `src-tauri/` (Cargo.toml, build.rs with `tauri_build::build()`, src/lib.rs, src/main.rs, tauri.conf.json, capabilities/, icons/) + frontend folder.
- IPC: `#[tauri::command]` fn → `generate_handler![...]` (one call); frontend `invoke` from `@tauri-apps/api/core`. Commands in modules must be `pub`; args camelCase; return `Result<T,E>` with serializable `E`. Large data → `tauri::ipc::Response`.
- `frontendDist` embeds all files (index.html entry); with `devUrl` unset Tauri serves `frontendDist` via its `tauri://localhost` custom protocol — **no dev server is started** (C11). Framework-agnostic; vanilla HTML/CSS/JS template exists.
- Window config (JSON keys are camelCase): `decorations:false`, `transparent:true`, `shadow:true`, `alwaysOnTop`, `windowEffects`, **`dragDropEnabled:false`** (Rust field `drag_drop_enabled` — required for HTML5 drag-drop on Windows, C3). Custom titlebar via `data-tauri-drag-region`.
- Plugins: `tauri-plugin-global-shortcut` (Rust ≥1.90, needs `allow-register` permissions), `tauri-plugin-autostart` (Rust ≥1.90), dialog plugin (`dialog:open`). Tauri tray API is `tauri::tray::TrayIconBuilder` + `tauri::menu::Menu` with `tauri::tray::TrayIconEvent::receiver()` — **not** the standalone tray-icon crate (C12).
- Capabilities: default-deny; a window matching no capability has **no IPC access**. Grant only what's needed. **Custom commands need explicit permissions** in `src-tauri/permissions/*.toml` (C21).
- Size: "< 600 KB" is a lower bound for a *minimal* app (C9); expect multi-megabyte with plugins + engine — measure in Phase 7. Release profile: `codegen-units=1`, `lto=true`, `opt-level="s"`, `panic="abort"`, `strip=true`.
- **Verdict (historical):** Tauri was recommended for exact visual fidelity. **Superseded by the user's 2026-11-20 decision** (§9): lowest RAM + fastest finish → egui + glow. Tauri's WebView2 process (100+ MB) and full systems reimplementation lost to egui's ~49 MB and already-working Win32 code. egui 0.37 **does** have `egui::Shadow`/`Frame::shadow()` (C8), so the visual gap is smaller than first thought.

### 11.4 rfd 0.18 / serde / tracing / thiserror
- `rfd` (**USED** — native file dialog for the egui app's Browse tab): sync `FileDialog::new().add_filter("exe",&["exe"]).pick_file() -> Option<PathBuf>`; async returns `Option<FileHandle>`. No feature needed on Windows. Add `rfd` to Cargo.toml in Phase 4.
- serde: `#[serde(default)]` (field or container), `#[serde(alias="old")]`, `#[serde(rename_all="camelCase")]`, `#[serde(from/try_from/into="V1")]` for migrations. **`#[serde(other)]` is a catch-all variant attribute for internally/adjacently tagged enums, NOT a version fallback** (C7) — use it inside a versioned-enum migration pattern. `deny_unknown_fields` incompatible with `flatten`.
- tracing: `tracing_subscriber::registry().with(EnvFilter::new("info")).with(fmt::layer().with_writer(file).with_ansi(false)).init()`; needs `env-filter` feature; use `try_init()` in tests.
- thiserror 2.x: `#[derive(Error)]` with `#[error("...")]`, `#[from]` (implies source), `#[error(transparent)]`. A `#[from]` variant must contain no other fields.

### 11.5 Tauri v2 tray / dialog / plugin APIs (added after audit — C12)
- **Tray (NOT the standalone tray-icon crate):** enable `tauri = { features = ["tray-icon"] }`; `tauri::tray::TrayIconBuilder` + `tauri::menu::{Menu, MenuItem, PredefinedMenuItem}`; events via `tauri::tray::TrayIconEvent::receiver()` (or `on_tray_icon_event` on the builder). Source: https://v2.tauri.app/learn/system-tray/
- **Dialog plugin:** `tauri-plugin-dialog`; Rust `app.dialog().file().blocking_pick_file()` or JS `open()` from `@tauri-apps/plugin-dialog`; permission `dialog:open`. Source: https://v2.tauri.app/plugin/dialog/
- **Global shortcut plugin:** `tauri-plugin-global-shortcut`; JS `register('CommandOrControl+Shift+C', cb)`; Rust `app.global_shortcut_manager().register("CTRL+U", cb)`; permissions `global-shortcut:allow-register` etc. Rust ≥1.90. Source: https://v2.tauri.app/plugin/global-shortcut/
- **Autostart plugin:** `tauri-plugin-autostart`; JS `enable()/isEnabled()/disable()`; Rust ≥1.90. Source: https://v2.tauri.app/plugin/autostart/
- **Custom command permissions:** define in `src-tauri/permissions/commands.toml` (`[[permission]]` with `commands.allow = [...]`, a `default` permission set) and reference from `capabilities/default.json`. Source: https://v2.tauri.app/develop/calling-rust/ + https://v2.tauri.app/reference/acl/capability

### 11.6 eframe/egui 0.37 APIs (CHOSEN STACK — for the UI rebuild, §9)
Source: docs.rs/eframe/0.37, docs.rs/egui/0.37 (fetched 2026-11-20).
- **App trait:** required method is `fn ui(&mut self, ui: &mut Ui, frame: &mut Frame)` (NOT `update`); `logic(&mut self, ctx, frame)` runs before each `ui` (no painting there); `raw_input_hook` (singular) can filter input. Source: https://docs.rs/eframe/0.37/eframe/trait.App.html
- **Panels:** `Panel::left/right/top/bottom`; **CentralPanel added LAST**; first panel added is outermost. Source: https://docs.rs/egui/0.37/egui/containers/panel/
- **Painting:** `ui.painter()` / `painter_at(rect)`; `rect_filled`, `rect_stroke`, `line_segment`, `text`, `Shape::gradient_rect`; coordinates are screen-space points (`pixels_per_point()` for DPI). Source: https://docs.rs/egui/0.37/egui/struct.Painter.html
- **Shadows:** `egui::Shadow { offset, blur, spread, color }` ("very similar to a box-shadow in CSS") + `Frame::shadow()` — hairlines via `Stroke` + `round_to_pixel_center()`. Source: https://docs.rs/egui/0.37/egui/struct.Shadow.html
- **Hit-testing custom regions:** `ui.interact(rect, id, sense)` → `Response` with `clicked()`, `secondary_clicked()`, `dragged()`, `hovered()`, `context_menu(|ui| ...)`, `dnd_set_drag_payload`/`dnd_hover_payload`/`dnd_release_payload`. Source: https://docs.rs/egui/0.37/egui/response/struct.Response.html
- **Modal:** `egui::containers::modal::Modal::new(id).show(ctx, |ui| ...)` — centered, backdrop blocks input, stacks. Source: https://docs.rs/egui/0.37/egui/containers/modal/struct.Modal.html
- **Text/shortcuts:** `TextEdit::singleline(&mut buf)`; `InputState::consume_shortcut(&KeyboardShortcut)`, `key_pressed(Key)`, `modifiers`. Source: https://docs.rs/egui/0.37/egui/widgets/text_edit/ + struct.InputState.html
- **Theme:** `Context::set_visuals`, `Visuals` (36 fields incl. `window_shadow`, `panel_fill`, `selection`, `warn_fg_color`); `ui.visuals_mut()`, `ui.style_mut()`. Source: https://docs.rs/egui/0.37/egui/style/struct.Visuals.html
- **Renderer:** keep `default-features=false, features=["glow",...]`; wgpu is default and caused ~323 MB RAM vs ~49 MB with glow. Source: https://docs.rs/eframe/0.37/eframe/

---

## 12. Current codebase audit (2026-11-20 — ground truth for Phases 1–2)

- **Files:** 7 `.rs` files, ~1,538 lines total. `editor.rs` 717, `engine.rs` 353, `config.rs` 194, `system.rs` 122, `overlay.rs` 93, `autostart.rs` 32, `main.rs` 27.
- **Dead dependency:** `egui_tiles 0.18` declared, never used — remove.
- **Config:** no `schema_version`, no `#[serde(default)]`, no migration. Parse errors silently wipe user data. `active()/active_mut()` use `.expect()` (config.rs:187,192). Config at `current_exe()` dir + `ScreenLayout.json`.
- **Hardcoded user data:** `config.rs:95–153` seeds 3 profiles with chrome/Obsidian/Slack/Spotify/URLs/macros; `editor.rs` has all UI strings, presets, and `"notepad.exe"` defaults (lines 362, 635). 12-color `PALETTE` duplicated in `overlay.rs` and `editor.rs`.
- **Hotkeys:** hardcoded in `system.rs:62–78`; `Profile.hotkey` field never consumed; Hotkeys view is read-only.
- **Safety:** 17 `unsafe` blocks with **zero** `// SAFETY:` comments; 19 `let _ =` swallowed results (mostly engine.rs Win32 calls); 8 `unwrap()` + 4 `expect()`.
- **Tests:** zero. No `#[test]`, no `tests/` dir. No TODO/FIXME markers.
- **Build:** release exe ~10 MB (README wrongly says ~6 MB); `Cargo.lock` present; glow renderer already configured (keep it — wgpu caused ~323 MB RAM vs ~49 MB).
- **Unused code:** `zone_editor_ui` in editor.rs is dead (never opened); `dirty()` re-reads/parses config from disk every frame.

---

## 13. Plan audit log (2026-11-20)

An independent subagent audited this plan against live official docs and the actual source. Verdict: intent and domain model sound, API research mostly accurate, but the plan was "an egui-era plan wearing a Tauri label." All 30 findings were applied. Summary of corrections:

| ID | Finding | Fix applied |
|---|---|---|
| C1 | "10 hotkeys" wrong — actually 17 keyboard + 1 mouse gesture | §2c count/namespaces, Phase 5 + §8 gates |
| C2 | windows features missing `Win32_System_Threading` | §11.1, Phase 2 |
| C3 | `drag_drop_enabled` is Rust field, JSON key is `dragDropEnabled` | Phase 3, §11.3 |
| C4 | Threading contradiction (dedicated thread vs main loop) | §10.1, Phase 2 — managers on main thread |
| C5 | engine-core scope overstated | Phase 2 — explicit pure vs Win32 split |
| C6 | Overlay + Ctrl+drag unresolved | Phase 0 GO/NO-GO spike |
| C7 | `#[serde(other)]` misdescribed | §11.4, Phase 1 |
| C8 | "egui has no box-shadow" false (`egui::Shadow` exists) | §9, §11.3 |
| C9 | "< 600 KB" is a floor, not a target | §9, §11.3 |
| C10 | rfd vs Tauri dialog plugin contradiction | Phase 4 — use Tauri dialog plugin |
| C11 | `devUrl` wording | Phase 3, §11.3 |
| C12 | Tray API cross-refs wrong | Phase 5, new §11.5 |
| C13 | Plan introduced hardcoded values | §2.2 expanded, §2b/§2d defaults |
| C14 | `Profile.hotkey` dead / two namespaces | §2c |
| C15 | Phase 0 removed only egui_tiles | Phase 0 — remove full egui stack |
| C16 | Exit gates not verifiable | Phase 3/5/6 gates made concrete |
| C17 | Dangling §11.8 ref | corrected to §11.3; §11.5 added |
| C18 | `dirty()` disk re-read | Phase 1 — in-memory snapshot |
| C19 | Config next to exe unwritable under Program Files | §2.3 location decision |
| C20 | Windows rename fails if dest exists | §2.3, Phase 1 — MoveFileExW |
| C21 | Custom commands need permissions | Phase 3, §11.5 |
| C22 | No rollback plan | Phase 0 rollback criteria |
| C23 | Existing config migration | §2.3, Phase 1 — v0 migration |
| C24 | Installer/signing/WebView2 mode | Phase 7 |
| C25 | AHK script detection | REMOVED per user 2026-11-20 — no in-app AHK check |
| C26 | App-picker enumeration detail | §2.2 (HKLM+HKCU) |
| C27 | Tauri-layer tests | §10.1 |
| C28 | panic=abort + unwraps | §10.1, Phase 0 — zero unwraps |
| C29 | §5 stale under Tauri | Phase 7 re-verification |
| C30 | egui wgpu default | §9, Phase 0 fallback note |

Claims verified CORRECT (no change): Tauri 2.12.7, Rust ≥1.90 for plugins, `dragDropEnabled` rationale, capabilities default-deny, IPC command rules, release profile flags, windows registry/DWM/WM constants, tray-icon/global-hotkey crate APIs, serde/thiserror/tracing syntax, egui `App::ui`, rfd sync API.

### Stack decision change (2026-11-20) — supersedes the Tauri recommendation
After the audit, the user set new priorities: **lowest RAM + fastest finish**. This reversed the stack choice:
- **Chosen: pure eframe/egui 0.37 + glow, no HTML/webview** (§9). Rationale: ~49 MB RAM vs WebView2's 100+ MB; Win32 systems code already written and working; no overlay feasibility spike needed.
- **Tauri v2 rejected** (§11.3): higher RAM, full systems reimplementation, overlay spike risk. Kept for reference only.
- **GPUI rejected** (§14): pre-1.0, frozen crate, maintainer deprioritized non-Zed work, no first-party tray/hotkeys/overlay widgets.
- Phases 3–5 rewritten from Tauri IPC/frontend to egui UI rebuild (§6). §11.6 added with egui 0.37 APIs. `rfd` is now actually used (Phase 4). The HTML mockups are design references only, never shipped.

---

## 14. GPUI evaluation (2026-11-20 — rejected)

The user asked about GPUI (gpui.rs, Zed's UI framework). Researched against official sources (gpui.rs, Zed README/examples, crates.io, community reports). **Rejected** — not suitable for this app/user.

- **Maturity:** v0.2.2 (2025-10-22), still latest; explicitly pre-1.0 with "often breaking changes." crates.io frozen ~8 months while `main` landed massive breaking changes (renderer swap, crate split) never released. "Tied to Zed" is explicit; a maintainer stated non-Zed work is deprioritized. Fragmented fork ecosystem (`gpui-ce`, `fc-gpui`, `open-gpui`, etc.).
- **Docs:** README says "best way to learn is to read the Zed source code." No official book. Unusual API (entities, contexts, actions). No first-party widgets (no text input, modal, context menu, undo) — DIY or third-party/GPL `ui` crate.
- **Windows:** works (D3D11 + DirectWrite, no features needed). But **no always-on-top or click-through in official `WindowOptions`** — the drag overlay needs raw Win32 or a fork. No first-party tray/global-hotkeys/autostart — separate crates + thread bridging.
- **Visual:** Tailwind-like styling (flex, grid, shadows, borders, tints) but "no total CSS compatibility" — cannot match `design-v3.html` pixel-for-pixel, same core reason egui was initially rejected.
- **Verdict:** GPUI's niche is an experienced Rust dev building a performance-sensitive custom-rendered app willing to pin a git rev or maintain a fork. None of those hold here. egui + glow wins on RAM, finish speed, and durability.

---

## 15. Implementation status (2026-11-20 — Phases 1–5 done)

Built with parallel subagents (phase jobs) + two audit/fix rounds. All work in `E:\Apps\Hotkeys\ScreenLayoutRust`.

| Phase | Status | Key files |
|---|---|---|
| 0 Foundation | ✅ | `Cargo.toml` (egui_tiles removed, glow kept, rfd/tracing added), `rust-toolchain.toml` |
| 1 Config | ✅ | `config.rs` — versioned schema, empty defaults, Settings + hotkeys, panic-free helpers, 5 tests |
| 2 Engine | ✅ | `engine_core.rs` (pure logic, 12 tests), `engine.rs` (SAFETY comments, tracing, axis cycling) |
| 3+4 UI | ✅ | `editor.rs` (~3400 lines) — full §2d UI: 3 panes, canvas, wizard, inspector, undo/redo, shortcuts, app picker, tools dock |
| 5 System | ✅ | `system.rs` (config hotkeys, Win→Meta), `autostart.rs`, `overlay.rs` (config tints/alphas) |

**Verified:** fmt clean · 23 tests pass · clippy clean · release build clean.

**Phase 6/7 now DONE:** auto-launch on startup (worker thread + mpsc, config-gated default off); display-change re-tiling via work-area polling at `display_debounce_ms`; path-based window matching (`find_window_by_path_min`, exe fallback); `dwm_snap_retries`/`min_window_px` wired from config; README corrected; CHANGELOG created; CI workflow created.

**Standardization complete (2026-11-21):** Workstreams A–C executed via 15 sequential subagent steps, each with a verification gate:
- **A (modularization):** 4,900-line `editor.rs` split into `src/editor/` with 13 focused modules (mod, state, theme, history, tokens, top_bar, toolbar, rail, canvas, inspector, action_bar, dialogs, views).
- **B (de-hardcoding):** all inline colors → theme tokens/config; 64 layout literals → `tokens.rs`; domain constants (resolutions, app cap, TTL, taskbar/stage heights) → config Settings. B4 audit CLEAN.
- **C (standardization):** `.gitignore` added; Cargo.toml metadata + windows deps gated; error handling cleaned (no panics/silent swallowing); 3 integration tests added (30 total); all 20 unsafe blocks have SAFETY comments; CI has cargo-audit; design HTML moved to `docs/design/`; README/CHANGELOG refreshed.

Final verification: fmt clean · 30 tests pass (27 inline + 3 integration) · clippy clean · release build clean · screenshot confirmed.

**Still deferred:** installer/signing; release-tag artifact attachment; i18n label centralization; full §8 GUI verification checklist (needs running the app with AHK stopped); further splitting the 4 files >600 lines (canvas 945, rail 783, dialogs 698, engine 666) — advisory only.

**Known limitations:** running-apps list is a 2s-cached snapshot; installed-app `.lnk` targets are guessed from shortcut names (Browse tab gives exact exe); "Launch" button is a console simulation (real execution via hotkeys); Unicode glyph icons instead of SVG (egui has no built-in SVG renderer).

### Close / startup / shutdown fix (2026-11-20)
- **Bug fixed:** app would not close. With default `close_action = Ask`, clicking X opened the dialog; clicking "Exit ZoneFlow" (remember unchecked) reset the action to `Ask` and sent `Close`, but the next frame's `close_requested()` re-opened the dialog and sent `CancelClose` — infinite loop. Tray "Exit" had the same problem. Fixed with an `exiting: bool` flag set on confirmed exit (dialog + tray + remembered Exit) that short-circuits the `close_requested` handler.
- **Graceful shutdown:** added `eframe::App::on_exit` — saves any dirty draft to config and logs. Tray/hotkeys are released when `System` drops.
- **Startup:** `main.rs` now initializes a `tracing` subscriber (file `zoneflow.log` next to exe, level via `RUST_LOG`, default `info`) so warnings are visible. `with_system()` loads config, tray and hotkeys at startup.
- **Note:** egui 0.37 uses `Id::unique("name")` for stable modal IDs — `Id::new` is deprecated (plan §7 was correct). Do not "fix" this again.
