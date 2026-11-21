# ZoneFlow Studio — Implementation Roadmap

**Status:** Planning document. No implementation changes are authorized by this file.
**Date:** 2026-11-21
**Basis:** Four independent audits + product/GUI/feature-preservation plans in this directory.

**Priority rule:** Correctness and reliability before visual polish. No large-scale rewrites. Keep the existing working backend. No new dependencies. Multi-monitor is out of scope.

---

## Phase 0 — Fix confirmed functional bugs (correctness first)

**Objective:** Make the features that exist actually work. No GUI changes.

| Task | Files | Requirement |
|---|---|---|
| Fix SendInput text injection | `engine.rs` | Replace ASCII-as-VK with `KEYEVENTF_UNICODE` for text; map `^X` to real VK; support common tokens. URLs and macros must type correctly. |
| Fix path-based launching | `engine.rs` | Launch with `zone.path` when non-empty (fallback to `zone.exe`); add `CREATE_NO_WINDOW`; keep 8s wait. |
| Fix drag-to-snap HWND | `engine.rs`, `editor/mod.rs` | Walk to top-level window (`GetAncestor(GA_ROOT)`); add AHK guards: class exclusion (taskbar/desktop), title-bar region, 8px threshold. |
| Fix Launch button | `editor/mod.rs`, `action_bar.rs` | Either perform real execution via `execute_zone_with_config` or rename to "Preview steps". Decision from product workflow §7 required. |
| Unify config source | `editor/mod.rs` | Hotkey actions must use in-memory config, not `config::load_config()` from disk; reload only on Revert. |
| Fix `LOADED_SETTINGS` staleness | `config.rs` | Pass `&Settings` explicitly or refresh snapshot on save/revert. |
| Fix `LaunchApp(i)` zone id | `editor/mod.rs` | Map hotkey index to zone id, not list position. |
| Fix profile switch apply | `editor/rail.rs`, `editor/mod.rs` | Rail switch = draft; Home/Tray switch = apply (after Home/tray exist). |
| Fix `restore_active` parity | `engine.rs` | Only re-snap when window is maximized (AHK parity). |
| Wire config retries into drag | `engine.rs`, `editor/mod.rs` | Drag path must use `settings.dwm_snap_retries`. |
| Fix docs on atomicity | `README.md`, `CHANGELOG.md` | Say "temp file + rename with small crash window", not "atomic". |

**Acceptance criteria:** Each bug has a regression test where feasible (unit test for key mapping, path selection, zone-id mapping); manual Windows test for drag-to-snap and launching. `cargo test` ≥ 30 pass; clippy/fmt clean.

**Dependencies:** None. Do this before any GUI work.

---

## Phase 1 — Application shell (layout ownership)

**Objective:** One component owns exact region rects; no panel guesses its siblings' sizes. Fixes clipping.

| Task | Files | Requirement |
|---|---|---|
| Add `Shell` component | NEW `editor/shell.rs`, `editor/mod.rs` | Divide viewport into TopBar / Body / StatusBar; Body into LeftRail / Main / RightAside; Main into Toolbar / Canvas / Footer. Use `egui::Panel` + explicit rect math — **no StripBuilder** (absent in egui 0.37). |
| Hand rects to children | `canvas.rs`, `rail.rs`, `inspector.rs`, `action_bar.rs` | Children receive their rect; remove `stage_lower_h`, `available_height()-56`, `48.0` guessing. |
| Add min window size | `main.rs` | `ViewportBuilder::with_min_inner_size([900,600])`. |
| Add status bar | NEW `editor/status_bar.rs`, `mod.rs` | Engine state, active profile, zone count, version. Move "Engine ready" out of top bar. |

**Acceptance criteria:** Screenshots at 1280×720, 1920×1080, 3840×2160 show no clipping; apps strip fully visible; status bar always visible. `cargo test` pass.

**Dependencies:** Phase 0 (so the shell isn't built on broken behavior).

---

## Phase 2 — Product workflow: Home, Apply/Save split, Disable, Tray

**Objective:** Implement the intended UX from `zoneflow-product-workflow.md`.

| Task | Files | Requirement |
|---|---|---|
| Add Home view | NEW `editor/home.rs`, `state.rs`, `mod.rs` | Active profile status, profile list with Apply/Edit/Duplicate/Rename/Delete, New profile, Disable layout. |
| Split Save and Apply | `action_bar.rs`, `mod.rs` | Separate buttons: Save (persist only), Apply (use now), Revert. Remove "Save & Apply". |
| Add Disable layout | `config.rs` (runtime flag), `mod.rs`, `system.rs`, tray | `layout_enabled` flag; disable suppresses re-tile/auto-launch/drag-snap; tray toggle; re-enable re-applies. |
| Tray profile switching | `system.rs` | Tray menu: active profile header, profile submenu (apply on click), Disable/Enable, Open editor, Settings, Exit. |
| Separate rail vs Home switching | `rail.rs`, `home.rs` | Rail switch loads draft; Home/Tray switch applies. |

**Acceptance criteria:** Manual Windows test of full journey: Open → Home → Edit → Assign → Save → Apply → Tray switch → Disable → Enable. No data loss. `cargo test` pass.

**Dependencies:** Phase 1 (shell must host Home view and status bar).

---

## Phase 3 — GUI polish (canvas, inspector, dialogs, responsive)

**Objective:** Make it look and feel like a polished utility. No new features.

| Task | Files | Requirement |
|---|---|---|
| Canvas refinement | `canvas.rs`, `theme.rs` | Lighter zone wash; whole-canvas empty state; readable footer typography; font sizes into theme. |
| Inspector contextual | `inspector.rs` | Show only when zone selected or collapsible; console collapsible/own tab; replace native labels with shadcn. |
| Dialogs responsive | `dialogs.rs` | Width from `ctx.viewport_rect()`; picker list fills height; no empty void. |
| Responsive behavior | `shell.rs`, `rail.rs`, `inspector.rs` | Use `window_at_least`/`container_at_least`; collapse right aside <1100px, left rail <800px; `CollapsingHeader` or conditional panels. |
| App library polish | `rail.rs` | Only heuristic tuning of existing junk filter; no rewrite. |
| Typography/spacing | `tokens.rs`, `theme.rs`, all views | Consistent scale; remove remaining magic numbers. |

**Acceptance criteria:** Screenshots at 2–3 window sizes; no competing regions; canvas is primary; secondary config contextual. `cargo test` pass.

**Dependencies:** Phases 1–2.

---

## Phase 4 — Cleanup, docs, verification

**Objective:** Leave a clean, honest, maintainable repository.

| Task | Files | Requirement |
|---|---|---|
| Dead code cleanup | `engine.rs`, `engine_core.rs`, `autostart.rs`, `system.rs` | Remove or wire dead wrappers (`is_auto_start`, `display_debounce_ms`, `show`, `CycleNext/Prev`, `findSharedBoundaries`) — only after call sites confirmed. |
| Duplicated defaults | `engine.rs`, `engine_core.rs`, `config.rs` | Single source of truth for snap points, min window, DWM retries. |
| Docs sync | `README.md`, `CHANGELOG.md`, plan file | Fix exe size (~14.2 MiB), remove stale Tauri language, correct test counts, edition rationale, icon claim. |
| Repo hygiene | `.gitignore`, `.gitattributes` | Ignore `inspect-*.png`; add line-ending rules; commit or clean the dirty tree. |
| Full verification | all | `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`; manual Windows test of all journeys. |

**Acceptance criteria:** Clean tree; all commands pass; docs match reality; no known broken features.

**Dependencies:** Phases 0–3.

---

## Task dependency graph

```
Phase 0 (bug fixes) ──► Phase 1 (shell) ──► Phase 2 (workflow) ──► Phase 3 (polish) ──► Phase 4 (cleanup)
```

No phase may start before the previous phase's acceptance criteria are met.

---

## Testing strategy

### Automated (every phase)
- `cargo fmt --check`
- `cargo test` (currently 30: 27 inline + 3 integration)
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --release`

### Regression tests to add (Phase 0)
- Key mapping: `^e` → Ctrl+E, URL text → correct characters (unit-testable mapping function).
- Path selection: launch uses `zone.path` when set.
- Zone-id mapping: hotkey index → zone id after splits/deletes.
- Config round-trip with new fields (extend `tests/config_file_roundtrip.rs`).

### Manual Windows testing (per phase)
- **Phase 0:** launch an app by full path; type a URL via rules; drag a window by its client area; Ctrl+drag near taskbar; Win+Down on maximized/non-maximized window.
- **Phase 1:** resize window to 1280×720 / 1920×1080 / maximized; verify no clipping; status bar visible.
- **Phase 2:** full user journey (Open → Home → Edit → Assign → Save → Apply → Tray switch → Disable → Enable).
- **Phase 3:** screenshots at multiple sizes; verify collapse behavior; dialogs proportionate.
- **Phase 4:** repeat all manual tests; verify docs against binary size and behavior.

---

## Risks

| Risk | Mitigation |
|---|---|
| Bug fixes change behavior users rely on | Preserve hotkeys/config; add regression tests; manual verification |
| Shell refactor breaks canvas interactions | Keep canvas painter; only change how rect is supplied; test drag/resize/draw after |
| "Lightweight" identity erodes (binary now ~14.2 MiB) | No new dependencies; measure size/RAM after each phase; avoid speculative abstractions |
| Stale plan/docs mislead future work | Phase 4 syncs all docs; this roadmap is the source of truth until then |
| Uncommitted tree causes confusion | Commit or clean before Phase 0 begins |
| Multi-monitor scope creep | Explicitly out of scope — do not implement or plan |

---

## Out of scope (decided)

- Multi-monitor support (primary work area only, matching AHK).
- New dependencies.
- Large-scale rewrites of working backend code.
- Triggers/functions/wallpaper features (DisplayFusion-style complexity).
- Removing any existing feature without explicit approval.
