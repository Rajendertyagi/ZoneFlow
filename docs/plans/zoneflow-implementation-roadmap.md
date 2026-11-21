# ZoneFlow Studio — Implementation Roadmap

**Status:** Planning document. No implementation changes are authorized by this file.
**Date:** 2026-11-21
**Basis:** Four independent audits + product/GUI/feature-preservation plans in this directory.

**Priority rule:** Correctness and reliability before visual polish. No large-scale rewrites. Keep the existing working backend. No new dependencies. Multi-monitor is out of scope. **Work in independent, coordinated workstreams — never integrate unfinished functionality without its acceptance criteria met.**

---

## 1. Workstreams (independent but coordinated)

| Workstream | Scope | Can start | Integrates at |
|---|---|---|---|
| **WS-A — Backend correctness & engine reliability** | Engine bugs (B1–B4, B10–B12); config snapshot (B7); atomic config writes (B11 code fix); dead wrappers. **No GUI functionality.** | Immediately | Checkpoint 1 |
| **WS-B — GUI shell, layout ownership & navigation** | Shell rect ownership, status bar, min window size, Home view, state indicators, responsive breakpoints | Immediately (independent of WS-A) | Checkpoint 2 |
| **WS-C — Profile management, assignments, Apply/Disable & tray workflows** | Save/Apply split, Launch & Arrange, Disable, tray switching, assignment UI, config-source unification (B6), workflow rules (D1–D6) | Start immediately; integrates at Checkpoint 3 (decisions D1–D6 are APPROVED) | Checkpoint 3 |
| **WS-D — Integration, performance & regression verification** | Performance baseline, automated tests, manual Windows journeys | Baseline immediately; full testing after each checkpoint | Continuous |

**Ownership correction:** GUI functionality must not be hidden inside the backend workstream. B5 (Launch button), B6 (config-source unification — all disk reloads are in `editor/mod.rs`/`system.rs`, not `engine.rs`), B8 (tray profile apply), and the docs fix are GUI/workflow tasks owned by WS-C, not WS-A. B9 is **not** a bug to fix in this plan (see §2) — current "by zone index" behavior matches the approved product spec. WS-A owns only engine/config correctness.

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
| Fix drag-to-snap HWND (B3) | `engine.rs` | Walk to top-level window (`GetAncestor(GA_ROOT)`) in `window_at`. Engine-side only. |
| Fix drag guards (B4) — engine helpers | `engine.rs` | Provide engine helpers for class exclusion, title-bar region and 8px threshold. **Call sites are in `Editor::poll_drag` (`mod.rs:400-444`) and must be wired by WS-B/WS-C** — pure "engine-side only" is not achievable. |
| Fix `LOADED_SETTINGS` (B7) | `config.rs` | Pass `&Settings` explicitly or refresh snapshot on save/revert; verify a runtime tunable edit propagates. |
| Fix `restore_active` parity (B10) | `engine.rs` | Re-snap only when the window is maximized; when **not** maximized, **minimize** it (AHK `Win+Down` parity, `ScreenLayout.ahk:283-284`). Both branches required. |
| Wire config retries into drag | `engine.rs` | Drag path must use `settings.dwm_snap_retries`. |
| Fix atomic config writes (B11 code) | `config.rs` | Replace remove-then-rename (`config.rs:390-402`) with a crash-safe write (write temp → flush → atomic rename without deleting the original first). Docs correction is WS-D. |
| Fix `is_auto_start()` (B12) | `autostart.rs` | **Remove** the dead helper and its false doc comment (wiring it would touch GUI `views.rs`, which is not WS-A scope). |

**Moved out of WS-A (GUI/workflow ownership):**
- B5 (Launch button real execution + Preview) → **WS-C** (D5).
- B6 (config-source unification) → **WS-C** — all disk reloads are in `editor/mod.rs:356-385` and `system.rs:47`, not `engine.rs`.
- B8 (tray profile apply) → **WS-C** (D6). Rail switching stays draft-load-only per D6.
- B9 (`LaunchApp(i)`) → **no change in this plan** — current "by zone index" behavior matches the approved product spec (`product-workflow.md:88`); switching to stable zone IDs is a separate future decision.
- Docs on atomicity (B11) → **WS-D** (docs sync at Checkpoint 4).

**Acceptance criteria:** each bug has a regression test where feasible (key mapping, path selection, B7 propagation, atomic write); manual Windows test for drag-to-snap and launching; `cargo test` ≥ 30 pass; clippy/fmt clean.

---

## 3. WS-B — GUI shell & navigation

**Objective:** One component owns exact region rects; canvas gets maximum space; no panel guesses siblings' sizes. Fixes clipping.

| Task | Files | Requirement |
|---|---|---|
| Add `Shell` component | NEW `editor/shell.rs`, `editor/mod.rs` | Divide viewport into TopBar / Body / StatusBar; Body into LeftRail / Main / optional RightAside; Main into Toolbar / Canvas / Footer. Use `egui::Panel` + explicit rect math — **no StripBuilder** (absent in egui 0.37). |
| Hand rects to children | `canvas.rs`, `rail.rs`, `inspector.rs`, `action_bar.rs` | Children receive their rect; remove `stage_lower_h`, `available_height()-56`, `48.0` guessing. |
| Canvas-focused layout | `shell.rs`, `canvas.rs` | Canvas gets max space; right aside appears only when a zone is selected or explicitly opened — no forced 3-column. |
| Library accessibility | `shell.rs`, `rail.rs` | Left rail toggleable/collapsible; library one click away. |
| Add min window size | `main.rs` | `ViewportBuilder::with_min_inner_size([700,500])` (logical points) — lowered from 900 so the defined 700–799pt responsive breakpoint is reachable (contradiction resolved). |
| Add status bar | NEW `editor/status_bar.rs`, `mod.rs` | Engine state, active profile, zone count, version. Move "Engine ready" out of top bar. |
| State indicators | `top_bar.rs`, `home.rs`, `status_bar.rs` | Four **independent** dimensions: Saved, Applied, Enabled, Selected/Editing — distinct indicators, never one ambiguous badge. |
| Add Home view | NEW `editor/home.rs`, `state.rs`, `mod.rs` | Active profile status, profile list with Apply/Edit/Duplicate/Rename/Delete, New profile, Disable layout. |

**Acceptance criteria (measurable):**
- Screenshots at 1280×720 / 1920×1080 / maximized show no clipping; verify all three responsive breakpoints (≥1100pt, 800–1099pt, 700–799pt) render without overlap.
- Rect-containment assertions: canvas rect lies within the CentralPanel rect and does not overlap the footer/status bar; rail and aside rects do not overlap the canvas.
- Library reachable in ≤1 click from any breakpoint (rail toggle or icon-only rail).
- Inspector appears only when a zone is selected or explicitly opened (no forced 3-column).
- Status bar visible with engine state, active profile, zone count, version; "Engine ready" removed from top bar.
- `cargo test` pass.

**Sequencing note:** the Applied and Enabled indicators (and the Home "Disable layout" button) depend on state fields (`applied_profile`, `layout_enabled`) introduced by WS-C (D4). At Checkpoint 2 these fields do not yet exist. WS-B must either add placeholder state or defer those two indicators and the Disable button to Checkpoint 3 — do not block Checkpoint 2 on WS-C state.

---

## 4. WS-C — Profile & application workflow

**Objective:** Implement the intended UX from `zoneflow-product-workflow.md`. **All six product decisions D1–D6 are APPROVED** — no decisions pending.

| Task | Files | Requirement |
|---|---|---|
| Split Save and Apply | `action_bar.rs`, `mod.rs` | Separate buttons: Save (persist only), Apply (use now), Revert. Remove "Save & Apply". |
| **(D1)** Apply + Launch & Arrange | `mod.rs`, `action_bar.rs` | Apply = arrange running apps only (default, never launch). Launch & Arrange = explicit launch + arrange, respect Launch/Attach, report per-zone failures. Two distinct actions. |
| **(D2)** Unsaved changes | `mod.rs`, `home.rs`, `dialogs.rs` | Disable Apply while dirty with "Save or revert your changes before applying"; profile switching with unsaved changes requires Discard/Cancel. |
| **(D3)** Replace assignment | `mod.rs`, `inspector.rs`, `dialogs.rs` | Preserve zone geometry/ID/position; replace identity; reset rules with confirmation for non-default; never delete library entry. |
| **(D4)** Disable layout | `config.rs` (runtime flag), `mod.rs`, `system.rs`, tray | Suspend all positioning, drag-snap, auto-launch, movement hotkeys, launch/focus hotkeys; tray/profile/editor/settings stay available; profile selection while disabled = select only; runtime-only, resets on restart; Enable re-applies via normal Apply. |
| **(D5)** Launch + Preview | `mod.rs`, `action_bar.rs`, `inspector.rs` | Launch performs real execution with success/failure feedback; separate Preview inspects steps without running; no simulated success. |
| **(D6)** Profile switching | `rail.rs`, `home.rs`, `system.rs` | Rail = load draft; Home = select only (explicit Apply/Launch & Arrange); tray = apply immediately when enabled, select-only when disabled. |
| Tray menu | `system.rs` | Active profile header, profile submenu, Disable/Enable, Open editor, Settings, Exit. |
| Assignment workflow | `rail.rs`, `dialogs.rs`, `mod.rs`, `inspector.rs` | Drag library→zone, picker assign, replace/remove (D3), contextual rule config, assignment visibility. |
| Same-app warning | `mod.rs`, `inspector.rs` | Detect duplicate exe/path within one profile; warn "only one window per app; last zone wins" (product §3b). No multi-window claim. |

**Acceptance criteria:** full user journey passes manually (Open → Home → Edit → Assign → Save → Apply → Launch & Arrange → Tray switch → Disable → Enable); no data loss; D1–D6 behaviors verified; unsaved-change rules honored; `cargo test` pass.

**Start / dependency:** WS-C can **start immediately** (decisions D1–D6 are approved, no blocking decisions) and works against the current backend. It **integrates at Checkpoint 3** on top of WS-A + WS-B, and its Home/tray UI needs the WS-B shell to be hosted. "Immediately" (start) and "Depends on WS-B" (integrate) are not contradictory — they refer to start vs. integration.

---

## 5. WS-D — Integration & regression testing

**Objective:** Prove correctness and keep the app lightweight.

### Performance measurement strategy
**Establish a baseline before redesigning.** No arbitrary targets — measure first.

| Metric | How to measure | Baseline (record before work starts) |
|---|---|---|
| Idle RAM | `Get-Process screenlayout \| Select-Object -ExpandProperty WorkingSet64` after 30s idle (repeatable; Task Manager "private working set" as cross-check) | Record actual (earlier claim ~49 MB with glow — verify) |
| Idle CPU | Sample `Get-Process screenlayout` CPU-time delta over a fixed 30s window (Task Manager % is low-precision for a near-zero idle app) | Record actual |
| Startup time | `Stopwatch` around process start, with an explicit readiness signal (poll for main window visibility) — **not** `Measure-Command`, which only times the launch call, not GUI readiness | Record actual |
| Release binary size | `Get-Item target/release/screenlayout.exe` | Known: 14,857,216 bytes (~14.17 MiB) — verify |

Rules: measure current build first; re-measure at each checkpoint; record deltas; no new dependencies without measuring impact; no numeric targets until ≥2 measurements exist. Store results in a dated table so deltas are comparable.

### Automated (every checkpoint)
- `cargo fmt --check`
- `cargo test` (currently 30: 27 inline + 3 integration)
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --release`

### Regression tests to add
- Key mapping (WS-A): `^e` → Ctrl+E, URL text → correct characters (unit-testable mapping function).
- Path selection (WS-A): launch uses `zone.path` when set (requires a test seam in `execute_zone_inner` so it can be checked without spawning a process).
- B7 propagation (WS-A): change a runtime tunable → verify it is observed after save/revert.
- Atomic write (WS-A): config write leaves the original intact if the process is interrupted mid-write.
- Config round-trip with new fields (WS-A): extend `tests/config_file_roundtrip.rs`.
- Same-app warning (WS-C): duplicate exe/path within one profile triggers the editor warning; no multi-window claim.
- B9 is intentionally **not** tested/changed — "by zone index" is the approved behavior.

### Manual Windows testing (per checkpoint)
- **Checkpoint 1 (WS-A):** launch an app by full path; type a URL via rules; drag a window by its client area; Ctrl+drag near taskbar; Win+Down on maximized/non-maximized window.
- **Checkpoint 2 (WS-B):** resize to 1280×720 / 1920×1080 / maximized; verify no clipping; canvas primary; status bar visible; library toggle works.
- **Checkpoint 3 (WS-C):** full journey (Open → Home → Edit → Assign → Save → Apply → **Launch & Arrange** → Preview → Tray switch → Disable → Enable); unsaved-change prompts; apply-failure reporting; same-app warning.
- **Checkpoint 4:** repeat all manual tests; verify docs against binary size and behavior; performance measurements recorded.

**WS-D acceptance criteria:** all automated commands pass at every checkpoint; every manual journey in the checkpoint list completes without data loss or clipping; performance baseline recorded before work starts and re-measured at each checkpoint with deltas logged; binary size and RAM show no unexplained regression; docs match the shipped binary (size, atomicity, test count). Any failing criterion blocks integration at that checkpoint.

---

## 6. Cleanup & docs sync (part of Checkpoint 4)

| Task | Files | Requirement |
|---|---|---|
| Dead code cleanup | `engine.rs`, `engine_core.rs`, `autostart.rs`, `system.rs` | Remove or wire dead wrappers only after call sites confirmed. |
| Duplicated defaults | `engine.rs`, `engine_core.rs`, `config.rs` | Single source of truth for snap points, min window, DWM retries. |
| Docs sync | `README.md`, `CHANGELOG.md`, plan files | Fix the two genuinely stale claims: exe size (~10 MB → ~14.2 MiB) and "atomic writes" (B11 — now actually fixed in code). Other items (Tauri negation, test counts 30 = 27+3, edition rationale, icon claim) are already correct — do not churn them. |
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
| Same app in multiple zones within one profile moves the same HWND twice (`engine.rs:315-334`, `:522-534`) | Warn in editor (product §3b); do not claim multi-window support; proper multi-window matching deferred |
| Responsive breakpoints unreachable | Minimum window size lowered to 700pt so 700–799pt breakpoint is reachable (GUI §2.3) |
| Repository branches | **Resolved (2026-11-21):** consolidated to a single `main` branch locally and remotely. Empty legacy `main` deleted, `master` renamed to `main` (history preserved), GitHub default set to `main`. No history rewrite or force-push. |

---

## 8. Out of scope (decided)

- Multi-monitor support (primary work area only, matching AHK).
- New dependencies.
- Large-scale rewrites of working backend code.
- Triggers/functions/wallpaper features (DisplayFusion-style complexity).
- Removing any existing feature without explicit approval.
- Data-model rewrite for profile-specific assignments — already supported (product §1a).
