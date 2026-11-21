# ZoneFlow Studio — Implementation Roadmap

**Status:** Planning document. No implementation changes are authorized by this file.
**Date:** 2026-11-21
**Basis:** Four independent audits + product/GUI/feature-preservation plans in this directory.

**Priority rule:** Correctness and reliability before visual polish. No large-scale rewrites. Keep the existing working backend. No new dependencies. Multi-monitor is out of scope. **Work in independent, coordinated workstreams — never integrate unfinished functionality without its acceptance criteria met.**

---

## 1. Workstreams (independent but coordinated)

| Workstream | Scope | Can start | Integrates at |
|---|---|---|---|
| **WS-A — Backend correctness** | The 12 confirmed bugs (B1–B12); config consistency; dead wrappers | Immediately | Checkpoint 1 |
| **WS-B — GUI shell & navigation** | Shell rect ownership, status bar, min window size, Home view, state badges | Immediately (independent of WS-A) | Checkpoint 2 |
| **WS-C — Profile & application workflow** | Save/Apply split, Disable, tray switching, assignment UI, workflow rules | After product decisions resolved | Checkpoint 3 |
| **WS-D — Integration & regression testing** | Performance baseline, automated tests, manual Windows journeys | Baseline immediately; full testing after each checkpoint | Continuous |

### Integration checkpoints

```
Checkpoint 1: WS-A bug fixes verified independently (backend stable)
Checkpoint 2: WS-B shell verified with current backend (no clipping, canvas primary)
Checkpoint 3: WS-C workflow integrated on top of WS-A + WS-B (full journey)
Checkpoint 4: WS-D full regression + performance + docs sync
```

At each checkpoint: run all automated commands, execute the relevant manual Windows tests, record results. Do not proceed with failing criteria.

---

## 2. WS-A — Backend correctness

**Objective:** Make the features that exist actually work. No GUI changes.

| Task | Files | Requirement |
|---|---|---|
| Fix SendInput text injection (B1) | `engine.rs` | Replace ASCII-as-VK with `KEYEVENTF_UNICODE` for text; map `^X` to real VK; support common tokens. URLs and macros must type correctly. |
| Fix path-based launching (B2) | `engine.rs` | Launch with `zone.path` when non-empty (fallback to `zone.exe`); add `CREATE_NO_WINDOW`; keep 8s wait. |
| Fix drag-to-snap HWND (B3/B4) | `engine.rs`, `editor/mod.rs` | Walk to top-level window (`GetAncestor(GA_ROOT)`); add AHK guards: class exclusion, title-bar region, 8px threshold. |
| Fix Launch button (B5) | `editor/mod.rs`, `action_bar.rs` | Perform real execution or rename to "Preview steps" — product decision §7 required. |
| Unify config source (B6) | `editor/mod.rs` | Hotkey actions use in-memory config, not `config::load_config()` from disk; reload only on Revert. |
| Fix `LOADED_SETTINGS` (B7) | `config.rs` | Pass `&Settings` explicitly or refresh snapshot on save/revert. |
| Fix profile switch apply (B8) | `editor/rail.rs`, `editor/mod.rs` | Rail switch = draft; Home/Tray switch = apply (after WS-C). |
| Fix `LaunchApp(i)` zone id (B9) | `editor/mod.rs` | Map hotkey index to zone id, not list position. |
| Fix `restore_active` parity (B10) | `engine.rs` | Only re-snap when window is maximized (AHK parity). |
| Wire config retries into drag | `engine.rs`, `editor/mod.rs` | Drag path must use `settings.dwm_snap_retries`. |
| Fix docs on atomicity (B11) | `README.md`, `CHANGELOG.md` | Say "temp file + rename with small crash window", not "atomic". |
| Fix `is_auto_start()` (B12) | `autostart.rs` | Wire or remove dead helper with false doc comment. |

**Acceptance criteria:** each bug has a regression test where feasible (key mapping, path selection, zone-id mapping); manual Windows test for drag-to-snap and launching; `cargo test` ≥ 30 pass; clippy/fmt clean.

---

## 3. WS-B — GUI shell & navigation

**Objective:** One component owns exact region rects; canvas gets maximum space; no panel guesses siblings' sizes. Fixes clipping.

| Task | Files | Requirement |
|---|---|---|
| Add `Shell` component | NEW `editor/shell.rs`, `editor/mod.rs` | Divide viewport into TopBar / Body / StatusBar; Body into LeftRail / Main / optional RightAside; Main into Toolbar / Canvas / Footer. Use `egui::Panel` + explicit rect math — **no StripBuilder** (absent in egui 0.37). |
| Hand rects to children | `canvas.rs`, `rail.rs`, `inspector.rs`, `action_bar.rs` | Children receive their rect; remove `stage_lower_h`, `available_height()-56`, `48.0` guessing. |
| Canvas-focused layout | `shell.rs`, `canvas.rs` | Canvas gets max space; right aside appears only when a zone is selected or explicitly opened — no forced 3-column. |
| Library accessibility | `shell.rs`, `rail.rs` | Left rail toggleable/collapsible; library one click away. |
| Add min window size | `main.rs` | `ViewportBuilder::with_min_inner_size([900,600])`. |
| Add status bar | NEW `editor/status_bar.rs`, `mod.rs` | Engine state, active profile, zone count, version. Move "Engine ready" out of top bar. |
| State badges | `top_bar.rs`, `home.rs`, `status_bar.rs` | Distinct visuals for saved / unsaved / applied / disabled. |
| Add Home view | NEW `editor/home.rs`, `state.rs`, `mod.rs` | Active profile status, profile list with Apply/Edit/Duplicate/Rename/Delete, New profile, Disable layout. |

**Acceptance criteria:** screenshots at 1280×720 / 1920×1080 / maximized show no clipping; canvas is primary; library accessible; inspector contextual; status bar visible; `cargo test` pass.

---

## 4. WS-C — Profile & application workflow

**Objective:** Implement the intended UX from `zoneflow-product-workflow.md` (§2a, §3a, §4a, §6).

| Task | Files | Requirement |
|---|---|---|
| Split Save and Apply | `action_bar.rs`, `mod.rs` | Separate buttons: Save (persist only), Apply (use now), Revert. Remove "Save & Apply". |
| Apply modes | `mod.rs`, `action_bar.rs` | Mode B (arrange running) default, Mode A (launch & arrange) optional — product decision §7. |
| Add Disable layout | `config.rs` (runtime flag), `mod.rs`, `system.rs`, tray | `layout_enabled` flag; exact suspend list from product §6; tray toggle; re-enable re-applies. |
| Tray profile switching | `system.rs` | Tray menu: active profile header, profile submenu (apply on click), Disable/Enable, Open editor, Settings, Exit. |
| Separate rail vs Home switching | `rail.rs`, `home.rs` | Rail switch loads draft; Home/Tray switch applies. |
| Assignment workflow | `rail.rs`, `dialogs.rs`, `mod.rs`, `inspector.rs` | Drag library→zone, picker assign, replace/remove rules (product §2a/§3a), contextual rule config, assignment visibility. |
| Ambiguity rules | `mod.rs`, `home.rs`, `dialogs.rs` | Unsaved-change prompts, apply-failure reporting, revert, replace/remove, same-app-in-multiple-zones warning (product §2a). |

**Acceptance criteria:** full user journey passes manually (Open → Home → Edit → Assign → Save → Apply → Tray switch → Disable → Enable); no data loss; unsaved-change rules honored; `cargo test` pass.

**Depends on:** product decisions (§7 of product workflow) + WS-B shell hosting Home/tray.

---

## 5. WS-D — Integration & regression testing

**Objective:** Prove correctness and keep the app lightweight.

### Performance measurement strategy
**Establish a baseline before redesigning.** No arbitrary targets — measure first.

| Metric | How to measure | Baseline (record before work starts) |
|---|---|---|
| Idle RAM | Task Manager "Memory (private working set)" after 30s idle | Record actual (earlier claim ~49 MB with glow — verify) |
| Idle CPU | Task Manager CPU % after 30s idle (should be near 0) | Record actual |
| Startup time | Launch → window interactive (stopwatch / `Measure-Command`) | Record actual |
| Release binary size | `Get-Item target/release/screenlayout.exe` | Known: 14,857,216 bytes (~14.17 MiB) — verify |

Rules: measure current build first; re-measure at each checkpoint; record deltas; no new dependencies without measuring impact; no numeric targets until ≥2 measurements exist.

### Automated (every checkpoint)
- `cargo fmt --check`
- `cargo test` (currently 30: 27 inline + 3 integration)
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --release`

### Regression tests to add (WS-A)
- Key mapping: `^e` → Ctrl+E, URL text → correct characters (unit-testable mapping function).
- Path selection: launch uses `zone.path` when set.
- Zone-id mapping: hotkey index → zone id after splits/deletes.
- Config round-trip with new fields (extend `tests/config_file_roundtrip.rs`).

### Manual Windows testing (per checkpoint)
- **Checkpoint 1 (WS-A):** launch an app by full path; type a URL via rules; drag a window by its client area; Ctrl+drag near taskbar; Win+Down on maximized/non-maximized window.
- **Checkpoint 2 (WS-B):** resize to 1280×720 / 1920×1080 / maximized; verify no clipping; canvas primary; status bar visible; library toggle works.
- **Checkpoint 3 (WS-C):** full journey (Open → Home → Edit → Assign → Save → Apply → Tray switch → Disable → Enable); unsaved-change prompts; apply-failure reporting.
- **Checkpoint 4:** repeat all manual tests; verify docs against binary size and behavior; performance measurements recorded.

---

## 6. Cleanup & docs sync (part of Checkpoint 4)

| Task | Files | Requirement |
|---|---|---|
| Dead code cleanup | `engine.rs`, `engine_core.rs`, `autostart.rs`, `system.rs` | Remove or wire dead wrappers only after call sites confirmed. |
| Duplicated defaults | `engine.rs`, `engine_core.rs`, `config.rs` | Single source of truth for snap points, min window, DWM retries. |
| Docs sync | `README.md`, `CHANGELOG.md`, plan files | Fix exe size (~14.2 MiB), remove stale Tauri language, correct test counts, edition rationale, icon claim. |
| Repo hygiene | `.gitignore`, `.gitattributes` | Ignore audit screenshots; add line-ending rules; keep tree clean. |

---

## 7. Risks

| Risk | Mitigation |
|---|---|
| Bug fixes change behavior users rely on | Preserve hotkeys/config; add regression tests; manual verification |
| Shell refactor breaks canvas interactions | Keep canvas painter; only change how rect is supplied; test drag/resize/draw after |
| "Lightweight" identity erodes (binary ~14.2 MiB) | No new dependencies; measure size/RAM at each checkpoint; avoid speculative abstractions |
| Stale plan/docs mislead future work | Checkpoint 4 syncs all docs; these four documents are the source of truth until then |
| Workstreams diverge | Integration checkpoints with explicit acceptance criteria; no unfinished integration |
| Multi-monitor scope creep | Explicitly out of scope — do not implement or plan |

---

## 8. Out of scope (decided)

- Multi-monitor support (primary work area only, matching AHK).
- New dependencies.
- Large-scale rewrites of working backend code.
- Triggers/functions/wallpaper features (DisplayFusion-style complexity).
- Removing any existing feature without explicit approval.
- Data-model rewrite for profile-specific assignments — already supported (product §1a).
