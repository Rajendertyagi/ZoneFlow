# ZoneFlow — Slint GUI Migration: Research & Implementation Plan

**Status:** Historical / reference research. **Retired as the active implementation direction on 2026-11-21.** The authoritative plan is `zoneflow-rust-axum-svelte-architecture.md`. This document is retained because its Slint feasibility findings, renderer/licensing analysis and benchmark methodology remain useful reference material. No implementation is authorized by this file.
**Date:** 2026-11-21
**Branch:** Based on actual latest `main` = `1976f67`. The task cited `9346ba1`; that is one commit behind (the audit-follow-up commit). No source differences relevant to this research exist between them.
**Basis:** Direct inspection of all `src/**/*.rs`, `Cargo.toml`, the five approved planning docs, the original `ScreenLayout.ahk`, official Slint documentation (slint.rs / docs.slint.dev / github.com/slint-ui/slint), and fresh performance measurements on this machine.

---

## 0. Goal and scope

**Goal:** Determine whether replacing egui/eframe with Slint would materially reduce ZoneFlow's RAM usage while delivering a modern, polished Windows interface — and, if so, define a safe migration path that preserves every existing feature and leaves the Rust backend untouched.

**Constraints carried into this plan:**
- Research/planning only. Do not replace egui, remove dependencies, or start migration from this document.
- Do not assume Slint is automatically lighter. No performance improvement is claimed here.
- No multi-monitor support (primary work area only, matching the AHK original).
- Existing Rust backend, Win32 engine, and config compatibility must be preserved.
- No WebView, web server, or extra runtime in the shipped app.

**Bottom line (detailed in §9):** Slint is technically feasible but **not worth adopting now**. The central reason is that the primary motivation (lower RAM) is unproven, while the cost — rewriting the ~1,000-line hand-painted canvas, which has no Slint equivalent — is certain and would put the "polished interface" goal at risk. A gating measurement spike is proposed before any migration decision is revisited.

---

## 1. Current architecture assessment

### 1.1 Shape of the application

The crate is a **binary** (`src/main.rs`, no `src/lib.rs`). Modules split cleanly into a toolkit-independent backend and a GUI seam.

| Layer | Files | GUI dependency | Migration impact |
|---|---|---|---|
| Pure geometry / mapping | `src/engine_core.rs` | **None** — only `use crate::config::{Profile, Zone}` | Unchanged |
| Win32 engine | `src/engine.rs` | **None** — imports `windows`, `config`, `engine_core` | Unchanged |
| Config schema + IO | `src/config.rs` | **None** — `serde`, `fs`, `OnceLock`/`RwLock` | Unchanged |
| OS integration | `src/system.rs`, `src/autostart.rs` | **None** — `global-hotkey`, `tray-icon`, Win32 registry | Unchanged (see §3.4) |
| App bootstrap | `src/main.rs` | `eframe`, `egui`, `shadcn-egui` | **Rewrite** |
| Editor UI | `src/editor/*.rs` (13 files, ~5,000 lines) | `egui`, `shadcn-egui`, `rfd` | **Rewrite** |
| Drag overlay | `src/overlay.rs` (162 lines) | `egui` viewports | **Rewrite** |

### 1.2 The backend/GUI seam is clean

Every type crossing the boundary is a plain owned serde struct with primitive/`String`/`Vec`/`BTreeMap` fields — **no egui types**: `AppConfig`, `Settings`, `Profile`, `Zone`, `ResolutionPreset`, `engine_core::Rect`, `KeyEvent`, `SharedBoundary`, `system::Action`. `AppConfig` is already `Send + Sync` and is cloned across a worker-thread boundary today (`editor/mod.rs:1118`). There are no `unsafe impl Send/Sync` hacks.

The GUI calls a thin, well-defined backend API:
- Config: `load_config`, `save_config`, `default_config`, `loaded_settings`, `resolutions`
- Engine: `execute_zone_with_config`, `retile`, `cycle_zone`, `cycle_zone_axis`, `maximize_active`, `restore_active`, `work_area`, `zone_pixels`, `compute_zones`, `zone_at`, `move_window_raw`, `drag_keys_held`, `cursor_pos`, `window_at`
- OS: `System::new`, `System::poll`, `System::register_from_config`, `autostart::set_auto_start`

**Conclusion:** the backend can be retained verbatim. The migration surface is confined to `main.rs`, `src/editor/`, and `src/overlay.rs`.

### 1.3 What the GUI actually does (inventory)

| Module | Lines | Responsibility |
|---|---:|---|
| `editor/mod.rs` | ~1,200 | `Editor` state, draft/dirty, zone & profile mutations, shortcuts, system-action dispatch, drag polling, startup/display parity, shell, frame loop |
| `editor/canvas.rs` | ~1,000 | Hand-painted monitor canvas: zone drawing, hit-testing, drag/resize, snap guides, context menu, merge/drop visuals, taskbar strip |
| `editor/rail.rs` | ~828 | App discovery (Start Menu / registry / tasklist, cleanup + dedup), library + profiles tabs, tools dock |
| `editor/dialogs.rs` | ~698 | Wizard (with live preview), app picker, help, reset confirm, text dialog, close dialog, toast |
| `editor/inspector.rs` | ~418 | Zone/automation inspector (Process / Browser / Macro tabs) + console |
| `editor/state.rs` | ~259 | Pure UI state types, templates, presets |
| `editor/toolbar.rs` | ~194 | Canvas toolbar (Layout, Draw, Undo/Redo, Split, Merge, Remove, Gap) |
| `editor/views.rs` | ~187 | Hotkeys + Settings views |
| `editor/action_bar.rs` | ~141 | Save/Revert/Launch actions + assigned-apps strip |
| `editor/theme.rs` | ~122 | `CanvasTheme` palette from shadcn + config tints |
| `editor/tokens.rs` | ~74 | Layout/color constants |
| `editor/history.rs` | ~64 | JSON-snapshot undo/redo |
| `editor/top_bar.rs` | ~55 | Brand, profile, resolution, dirty badge |
| `overlay.rs` | ~162 | Always-on-top, mouse-passthrough drag overlay viewport |

**The dominant cost is `canvas.rs`**: ~1,000 lines of immediate-mode `egui::Painter` drawing (bezel/shadow, grid, snap guides, tinted zone fills, strokes, accent bars, badges, truncated text, pills, %/px footers, resize handles, HUD, taskbar strip) plus custom hit-testing (`zone_at_percent`, `boundary_at`). This is the single hardest thing to port.

### 1.4 Threading and event model

- `eframe::run_native` owns the Win32 message loop. Each frame `Editor::logic()` polls `System::poll()` (hotkey/tray events), drag state, the exec console, auto-launch results, and display changes.
- One worker thread exists: `maybe_auto_launch()` (`mod.rs:1139`) clones `AppConfig`, runs `execute_zone_with_config`, and returns results over `mpsc`.
- Global statics: `config::LOADED_SETTINGS` (`OnceLock<RwLock<Settings>>`), `rail::APPS` (`OnceLock<Vec<AppEntry>>`), `Arc<RwLock<OverlayState>>` shared with the overlay viewport.
- Engine calls are synchronous on the UI thread and can block up to 8 s (`execute_zone_inner` wait loop). Any replacement UI must keep a regular event loop that calls `System::poll()` frequently and must preserve the worker-thread pattern for auto-launch.

### 1.5 Dependencies

**GUI-only (would be removed in a migration):** `eframe 0.37` (glow), `egui 0.37`, `shadcn-egui 0.8` (JetBrains Mono font), `rfd 0.18` (file picker).

**Backend / OS (must stay):** `serde`, `serde_json`, `tracing`, `tracing-subscriber`, `global-hotkey 0.10`, `tray-icon 0.26`, `windows 0.62.2` (8 Win32 features). `Win32_UI_Shell` and `Win32_System_SystemInformation` appear unused directly and are likely transitive — verify before touching.

Release profile: `opt-level = 2`, `lto = true`, `strip = true`. Edition 2021 is deliberate (edition 2024 makes `unsafe_op_in_unsafe_fn` deny-by-default, breaking the Win32 `extern "system"` callbacks in `engine.rs:275`). **Do not change edition as part of a GUI migration.**

### 1.6 Measured current baseline (this session, commit `1976f67`)

Measured with the existing `docs/performance/baseline.ps1` on the same machine used for all comparisons. Three consecutive runs:

| Metric | Run 1 | Run 2 | Run 3 | Reported |
|---|---|---|---|---|
| Release exe size | — | — | — | **14,901,248 bytes (14.21 MiB)** |
| Cold startup → window visible | 442.3 ms | 197.3 ms | 195.6 ms | **~195–440 ms (noisy)** |
| Idle working set | 109.4 MiB | 111.8 MiB | 111.4 MiB | **~109–112 MiB** |
| Idle private bytes | 58.8 MiB | 61.6 MiB | 61.3 MiB | **~59–62 MiB** |
| Idle CPU | 0.47% | 0.26% | 0.36% | **< 0.5%** |
| Active working set | 113.5 MiB | 115.2 MiB | 114.6 MiB | **~113–115 MiB** |
| Active private bytes | 63.0 MiB | 64.8 MiB | 64.2 MiB | **~63–65 MiB** |
| Active CPU | 1.01% | 46.14% | 43.89% | **highly variable** |

Environment: Windows 11 Pro 10.0.26300, i3-12100 (4c/8t), 31.6 GB RAM, rustc 1.99.1 MSVC, eframe/egui 0.37 + glow. Method: 50 ms window-visibility polling for startup, 30 s idle, 20 s synthetic cursor movement for active. Startup and active CPU are noisy; RAM figures are stable. The earlier `docs/performance/baseline.md` (commit `363201c`) recorded ~120 MiB WS / ~61 MiB private and 14,857,216 bytes — consistent within run-to-run variation.

**Important:** `README.md:3,29` still claims "~10 MB exe / ~49 MB private RAM" and "30 tests". Those figures are stale; the measured values above are authoritative. Correcting them is a docs task, not a migration task.

---

## 2. Slint feasibility and limitations

### 2.1 Facts (official sources, as of 2026-11-21)

- **Current version:** Slint 1.19.0 (released 2026-11-16). Crates: `slint = "1.19.0"`, `slint-build = "1.19.0"`.
- **Rust integration:** three options — inline `slint::slint!{}` macro; external `.slint` files compiled by `slint-build` in `build.rs` with `slint::include_modules!()`; or the `slint_interpreter` runtime API. Generated components expose `new()`, `set_/get_<prop>`, `on_<cb>`, `invoke_<cb>`, and `ComponentHandle` methods. Globals via `global::<T>()`.
- **Models/views:** `Model`, `ModelRc`, `VecModel`, `FilterModel`, `SortModel`, `MapModel`, `ListView`, `StandardTableView`.
- **Event loop:** must run on the main thread. Worker→UI communication via `invoke_from_event_loop()`; `slint::spawn_local()` for futures; `Timer` for periodic callbacks; `BackendSelector` or `SLINT_BACKEND` for renderer choice.
- **Renderers:** software (CPU, no GPU, smallest footprint, but no rotation/scaling, no inner shadows, western-scripts-only text, no `Path` with line-by-line rendering); FemtoVG (OpenGL ES 2.0, default, "text and path rendering quality sometimes sub-optimal"); Skia (richest, "heavy disk-footprint", fussy Windows build); Vello (experimental, GPU compute only); Qt (optional, QPainter). Default is winit + FemtoVG, with software also compiled in.
- **No WebView / no web server / no GPU requirement.** `.slint` markup compiles to native code; the software renderer needs no GPU. (An optional `mcp` debug feature can start an HTTP server — not for production.)
- **Windows:** winit backend tested on Windows 10/11 x86-64 and aarch64. Logical pixels + `scale_factor()`. `Window` supports `background` (translucent brush), `always-on-top`, `no-frame`, `resize-border-width`. Custom chrome via `WindowMoveArea`. **System tray is built in** (`SystemTrayIcon`, `Shell_NotifyIcon` on Windows). **Global hotkeys are NOT built in** — in-app `KeyBinding` (since 1.16) requires focus; system-wide shortcuts remain a non-goal (issue #102) and must use Win32/`global-hotkey`.
- **Licensing:** triple-licensed — Royalty-free 2.0 (proprietary desktop/mobile/web **at no cost**, embedded excluded), GPLv3 (open source, incl. embedded), or commercial. For a closed-source Windows desktop app, Royalty-free 2.0 is free with **one obligation**: display the `AboutSlint` widget in an About screen/splash, **or** display the Slint attribution badge on a public download page. No source disclosure. License terms stable since Royalty-free 2.0 (May 2024).
- **Deployment:** release build embeds compiled `.slint`, images and fonts; depends only on system libraries. MSVC builds link the C runtime dynamically → Visual C++ Redistributable needed unless CRT is statically linked. Skia additionally needs the C++ standard library. No ICU/vcpkg requirement for desktop Rust builds.
- **Memory claims:** the only official figure is "the Slint runtime fits in less than 300 KiB RAM" — this refers to the embedded runtime core, **not** a desktop app with a renderer. **No credible Slint-vs-egui RAM or binary-size comparison exists in official sources.** Any specific "Slint uses X MB vs egui Y MB" claim would be invented.

Sources: [slint.rs](https://slint.rs/), [Rust API](https://docs.slint.dev/latest/docs/rust/slint/), [cargo features](https://docs.slint.dev/latest/docs/rust/slint/docs/cargo_features/index.html), [backends & renderers](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/), [Windows guide](https://docs.slint.dev/latest/docs/slint/guide/platforms/desktop/windows/general/), [packaging](https://docs.slint.dev/latest/docs/slint/guide/platforms/desktop/packaging/windows/), [LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md), [Royalty-free 2.0](https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md), [SystemTrayIcon](https://docs.slint.dev/latest/docs/slint/reference/window/systemtrayicon/), [Path](https://docs.slint.dev/latest/docs/slint/reference/elements/path/), [Image](https://docs.slint.dev/latest/docs/slint/reference/elements/image/), [drag & drop](https://docs.slint.dev/latest/docs/slint/guide/development/drag-and-drop/), [issue #102](https://github.com/slint-ui/slint/issues/102), [1.19 blog](https://slint.dev/blog/slint-1.19-released.html), [desktop-ready blog](https://slint.dev/blog/making-slint-desktop-ready.html).

### 2.2 Limitations that matter for ZoneFlow

1. **No immediate-mode canvas — the critical gap.** Slint has no `Canvas` element that Rust can paint per frame. Workarounds:
   - `Path` element (declarative SVG-like shapes) — good for static zones/guides, but `commands` is write-only and shapes are described in markup, not painted from Rust.
   - `Image` + `slint::Image` from a CPU pixel buffer (`SharedPixelBuffer`) — the main workaround: paint tints/shadows/hairlines in Rust, upload the whole buffer each frame. Cost: full-buffer re-upload per frame; no GPU-accelerated arbitrary drawing.
   - `Window::set_rendering_notifier()` — lower-level, backend-specific escape hatch for custom GPU rendering.
   This means `canvas.rs` (~1,000 lines) and the wizard live preview and overlay painting must be **redesigned**, not translated.
2. **Renderer trade-off conflicts with the goal.** The renderer most likely to reduce RAM (software) is the one with the most visual limitations and CPU cost. FemtoVG is GPU-backed like glow, so RAM may not improve. Skia is explicitly heavy on disk footprint.
3. **Global hotkeys still need `global-hotkey`** — no benefit there; that backend code stays.
4. **Desktop maturity is recent.** Slint's "make it production-ready for desktop" effort began October 2025; tray, in-app shortcuts, drag-and-drop and modal windows only landed in 1.16–1.19 (2026).
5. **Menus are not fully customizable** (per a commercial DAW developer in issue #102) — relevant to the tray menu and context menus.
6. **Native look is being deprecated** — Fluent is default since 1.16; other styles deprecated. "Modern, polished" would come from custom Slint styling, not native widgets.
7. **Ecosystem and learning curve** — a bespoke DSL with its own types, bindings, states and compiler, separate from Rust; far smaller ecosystem than egui.
8. **Windows gotchas** — console window by default (`#![windows_subsystem = "windows"]`), debug-build stack overflows needing `/STACK:8000000`, Skia path-length/MSVC constraints.
9. **Accessibility** via AccessKit is enabled by default but less mature than long-standing frameworks. Text rendering has been catching up (swash rasterization in 1.16; rich text only since 1.16).
10. **Licensing obligation** — the AboutSlint widget or a public attribution badge must be added. Minor, but non-zero.

### 2.3 Feasibility verdict

Slint is **technically feasible** for ZoneFlow: it supports native Windows windows, a built-in tray, drag-and-drop, context menus, models/lists, text inputs, dialogs, and translucent frameless windows. The backend can remain untouched. But the two headline goals are in tension: the renderer that might save RAM degrades visual polish, and the component that defines the app's visual identity (the canvas) is exactly the one with no direct Slint equivalent.

---

## 3. Proposed architecture (only if the §5 spike passes the gate)

### 3.1 Principle

Keep 100% of the backend. Replace only the three GUI files/layers. Mirror the draft `AppConfig` into Slint properties/models; route UI callbacks back into the same `Editor`-style Rust state object. No new runtime, no web server, no WebView.

### 3.2 Component structure

```
src/
  main.rs                  bootstrap: logging, Slint event loop, renderer selection
  backend/                 (unchanged: engine_core, engine, config, system, autostart)
  ui/
    app.slint              root Window: shell regions, view switching
    shell.slint            top bar / body / status bar region ownership
    canvas.slint           zone stage: Image host + TouchArea/DropArea overlay
    canvas_paint.rs        Rust: paint zones/tints/shadows/guides into SharedPixelBuffer
    rail.slint             app library + profiles tabs
    dialogs.slint          wizard, picker, help, confirms, close dialog
    inspector.slint        zone/automation tabs + console
    views.slint            hotkeys + settings
    toolbar.slint, action_bar.slint, top_bar.slint, status_bar.slint
    overlay.slint          frameless always-on-top drag overlay window
    theme.slint            colors, spacing, typography tokens
    bridge.rs              Rust state: draft config, dirty flag, history, callbacks
    app_discovery.rs       moved from rail.rs (reg/tasklist/.lnk) — pure Rust, no UI
```

### 3.3 State bridge

- Keep `draft: AppConfig`, `last_saved_json`, `history`, `selected_zone`, and the four independent state dimensions (Saved / Applied / Enabled / Selected) in Rust (`bridge.rs`), exactly as today.
- Expose to Slint as properties (`<i32>`, `<bool>`, `<string>`) and `Model`s (`VecModel` of zone/app rows). Use `set_*` from Rust after every mutation; use `on_*` callbacks from Slint to call the same mutation methods (`split_zone`, `attach_app`, `save`, `revert`, …).
- Keep `history.rs` unchanged — JSON snapshots are toolkit-agnostic.
- Keep `app_discovery.rs` as pure Rust (moved out of `rail.rs`); feed results into a Slint `VecModel`.

### 3.4 Event loop and OS integration

- `slint::run_event_loop()` on the main thread; create all components there.
- Drive `System::poll()` with a `slint::Timer` at a short interval (e.g. 16–50 ms) to keep hotkey/tray latency comparable to the current per-frame poll.
- Keep `global-hotkey` and `tray-icon` crates as-is — they are toolkit-independent and already work. **Do not switch to Slint's `SystemTrayIcon` in the first migration**; it would add churn without reducing memory. Revisit only if the built-in tray proves equivalent.
- Keep the auto-launch worker thread; deliver results with `slint::invoke_from_event_loop()`.
- Close-to-tray behavior: Slint `Window` close callback → same `CloseAction` enum logic; hide the window instead of exiting. **Verify Slint's window-hide/show and close-request semantics on Windows in the spike** — this is a known integration seam today.

### 3.5 Renderer choice

Start with **software** for the memory spike (lowest footprint, no GPU), then evaluate **FemtoVG** for visual quality. Avoid Skia unless its rendering quality is proven necessary — its disk footprint and Windows build complexity work against the goal. Make the choice via Cargo features + `BackendSelector`, not an environment variable, in the shipped app.

### 3.6 Open questions to resolve in the spike (not assumptions)

- Does Slint's winit backend support **mouse-passthrough** for the drag overlay on Windows? The official `Window` reference lists `always-on-top`, `no-frame`, `background`, `resize-border-width` but **does not document mouse passthrough**. If unsupported, the overlay must be implemented as a Win32 layered window owned by Rust — a custom implementation (§8).
- Can `Image`-from-pixel-buffer repaints sustain the canvas at acceptable CPU and latency at 1920×1080?
- Does the software renderer's lack of inner shadows / rotation materially change the zone visuals?

---

## 4. Existing feature → Slint mapping

| Required feature | Current implementation | Slint approach | Effort / risk |
|---|---|---|---|
| Visual zone creation / resize / split / merge | `canvas.rs` painter + `mod.rs` mutations + `engine_core` geometry | `canvas.slint` hosts an `Image`; `canvas_paint.rs` paints zones into a pixel buffer; `TouchArea` for hit-testing; `Path`/`Image` for guides; mutations call unchanged `mod.rs`-equivalent methods | **Very high** — no direct equivalent; largest rewrite |
| Drag-and-drop app assignment | egui `dnd_set_drag_payload` / `dnd_release_payload` | Slint `DragArea`/`DropArea` with data-transfer, or `TouchArea` pointer events for in-canvas drags | Medium — semantics differ; needs testing |
| Profiles and switching | `rail.rs` profiles tab + `config.rs` | `rail.slint` `ListView` bound to a `VecModel`; same `load_config`/`save_config` | Low–medium |
| Save / Apply / Disable / Revert | `action_bar.rs`, `mod.rs`, `system.rs` | Buttons in `action_bar.slint`/`status_bar.slint` calling the same Rust verbs; Disable is a runtime flag in the bridge | Low — logic unchanged |
| App discovery and picker | `rail.rs` enumeration + `dialogs.rs` picker + `rfd` | Move enumeration to `app_discovery.rs` (pure Rust); `dialogs.slint` tabs bound to models; keep `rfd` | Medium — UI rewrite, data layer reused |
| Inspector, settings, hotkeys | `inspector.rs`, `views.rs` | `inspector.slint`, `views.slint` with `LineEdit`/`ComboBox`/`CheckBox`/`Slider`; same config fields | Medium — many small widgets |
| System tray + background | `system.rs` (`tray-icon`) | Keep `tray-icon` unchanged; map `Action`s into bridge callbacks | Low |
| Window management / Win32 engine | `engine.rs`, `engine_core.rs` | **Unchanged** | None |
| Config compatibility | `config.rs` schema v1 | **Unchanged** — same JSON, same migration | None |
| Drag-to-snap overlay | `overlay.rs` egui viewport | `overlay.slint` frameless window, or Win32 layered window if passthrough unsupported | **High** — verify in spike |
| Undo/redo | `history.rs` JSON snapshots | **Unchanged** in Rust; toolbar buttons in Slint | Low |
| Console / execution feedback | `mod.rs` `run_sequence` + `inspector.rs` console | Bridge keeps log buffer; `inspector.slint` binds a model | Low–medium |
| Theme / tokens | `theme.rs`, `tokens.rs`, shadcn palette | `theme.slint` tokens; port palette values | Medium — visual parity work |

---

## 5. Performance benchmark plan

### 5.1 Principle

Measure, do not predict. No Slint number is asserted in this document. The current egui numbers in §1.6 were measured on this machine in this session; the Slint numbers must be measured on the **same machine, same day, same durations, same method** before any decision.

### 5.2 Phase 0 — gating spike (no application source changes)

Build a **standalone** Slint prototype outside the application tree (e.g. a scratch crate in `D:\Coding\OpenCode\user\tmp\opencode` or a throwaway branch). It must include:
- A window of the same default size (1500×850) with the same title.
- A canvas rendering the same 3-zone layout with tints, strokes, hairlines and a snap guide — using the chosen renderer's realistic path (`Image`-from-pixel-buffer for software/FemtoVG).
- A list of ~200 app entries (matching `app_list_cap`) in a scroll view.
- A tray icon and a periodic timer that drives repaint, mirroring the current frame loop.
- No backend, no config IO — only the GUI shell that determines toolkit overhead.

Build in release with the same profile (`opt-level=2`, `lto=true`, `strip=true`).

### 5.3 Workloads and metrics

Adapt `docs/performance/baseline.ps1` into a paired script that runs both executables with identical parameters:

| Metric | Method | Notes |
|---|---|---|
| Idle working set / private bytes | 30 s idle, `WorkingSet64` / `PrivateMemorySize64` at window end | Stable in current app; primary comparison |
| Active working set / private bytes | 20 s deterministic repaint workload (timer-driven zone highlight cycling, not random cursor) | Deterministic workload avoids the current active-CPU noise |
| CPU | `TotalProcessorTime` delta / wall time over the same windows | Report idle and active separately |
| Cold startup | `Stopwatch` from `Start-Process` to visible main window, 50 ms polling | Report as a range; ±50 ms granularity |
| Release exe size | `Get-Item target/release/*.exe` | Record bytes and MiB |

Run **≥3 consecutive runs per executable per renderer**, report ranges, and keep the raw JSON. Test Slint with both the software and FemtoVG renderers. Record the exact Slint version, Cargo features, and renderer selected.

### 5.4 Decision gate

Proceed to migration **only if** the spike shows a material, repeatable reduction in private bytes and working set **with acceptable visual parity** — and the product owner agrees the reduction justifies the rewrite cost. **The threshold must be agreed before the spike is run; this plan does not invent one.** If the reduction is marginal, within run-to-run noise, or only achievable with the visually-limited software renderer, stop and pursue the alternative in §9.

---

## 6. Migration phases and integration checkpoints

Each phase ends with a checkpoint. A failing checkpoint blocks the next phase.

### Phase 0 — Decision spike
- Build the standalone Slint prototype (§5.2); run the paired benchmark (§5.3).
- **Checkpoint 0:** gate decision recorded in `docs/performance/slint-spike.md` with raw numbers. If gate fails → stop; no migration.

### Phase 1 — Scaffolding and shell
- Add `slint`/`slint-build`, `build.rs`, `.slint` files; port `main.rs` bootstrap (logging retained, renderer selected explicitly); implement `shell.slint`, `top_bar.slint`, `status_bar.slint`; move `app_discovery.rs` out of `rail.rs`.
- **Checkpoint 1:** app builds and shows the shell with correct region ownership; `cargo fmt/clippy/test` clean; backend tests unchanged and passing; no clipping at 1280×720 / 1920×1080 / maximized.

### Phase 2 — Canvas (highest risk, do first among features)
- Implement `canvas.slint` + `canvas_paint.rs`; port hit-testing, drag/resize, snap guides, draw mode, context menu, merge/drop visuals.
- **Checkpoint 2:** all canvas journeys pass manually (create, resize, split, merge, draw, drag-app-to-zone, snap guides); visual parity screenshots reviewed; no regression in `engine_core` geometry tests. **If canvas parity cannot be achieved, stop — this is the core of the app.**

### Phase 3 — Rails, dialogs, inspector, views
- Port `rail.slint`, `dialogs.slint`, `inspector.slint`, `views.slint`, `toolbar.slint`, `action_bar.slint`; wire all callbacks to the unchanged Rust verbs.
- **Checkpoint 3:** full user journey passes (Open → Home → Edit → Assign → Save → Apply → Launch & Arrange → Tray switch → Disable → Enable); unsaved-change rules honored; no data loss.

### Phase 4 — Overlay, tray, close behavior
- Implement `overlay.slint` or the Win32 layered-window fallback; verify `System::poll()` timer cadence; port close-to-tray and `CloseAction` handling.
- **Checkpoint 4:** Ctrl+drag overlay works with correct zone highlight; global hotkeys fire while unfocused; tray menu works without the window; close/minimize/exit behave as before.

### Phase 5 — Verification, performance, cleanup
- Re-run the paired benchmark against the migrated app; remove egui/eframe/shadcn-egui dependencies; update `README.md`, `CHANGELOG.md`, and all planning docs; add the Slint attribution (AboutSlint widget or download-page badge).
- **Checkpoint 5:** all automated checks clean; performance deltas recorded with no unexplained regression; docs match the shipped binary; licensing obligation verified.

---

## 7. Risks, dependencies, testing

### 7.1 Risks

| Risk | Severity | Mitigation |
|---|---|---|
| No RAM benefit (the premise is unproven) | **High** | Phase 0 gate before any source change; stop if marginal |
| Canvas parity unachievable / visual regression | **High** | Phase 2 checkpoint with screenshot review; stop condition |
| Software renderer limitations degrade "polished" goal | High | Benchmark both renderers; choose with visual evidence |
| Overlay mouse-passthrough unsupported on Windows | Medium–high | Validate in Phase 0; Win32 layered-window fallback (§8) |
| Global hotkeys regress | Medium | Keep `global-hotkey` unchanged; Phase 4 checkpoint |
| Tray/menu customization limits | Medium | Keep `tray-icon` initially; custom menus if needed |
| Licensing attribution missed | Low | Explicit Phase 5 task |
| DSL learning curve / slower iteration | Medium | Phase 1 spike includes a real component, not a toy |
| Binary size grows (Skia) | Medium | Avoid Skia unless proven necessary; measure at Phase 5 |
| Stale docs mislead | Low | Phase 5 docs sync |
| Scope creep (multi-monitor, backend rewrites) | Medium | Explicitly out of scope; backend frozen |

### 7.2 Dependencies

- **Add:** `slint 1.19.0`, `slint-build 1.19.0` (build-dependency), `build.rs`.
- **Remove (only after Phase 5):** `eframe`, `egui`, `shadcn-egui`, `rfd` (if Slint's file dialog is not used — otherwise keep `rfd`).
- **Keep:** all backend/OS crates and the `windows` feature set. Do not change edition 2021.

### 7.3 Testing strategy

- **Unchanged:** all 43 existing tests (40 unit + 3 integration) — they exercise the backend and must pass at every checkpoint.
- **New:** bridge tests for draft→property mirroring, dirty flag, callback→mutation routing, and model updates; these are pure Rust and testable without a UI.
- **Manual Windows journeys:** the full list in `zoneflow-implementation-roadmap.md` §5, plus Slint-specific checks (renderer selection, DPI scaling, overlay passthrough, tray without window).
- **Performance:** paired benchmark at Phase 0 and Phase 5; raw JSON retained; no target invented.

---

## 8. Features requiring custom implementation

These have no direct Slint equivalent and must be built explicitly:

1. **Zone canvas painter** — `Image` from CPU pixel buffers or `set_rendering_notifier`; port of ~1,000 lines of `egui::Painter` work including tints, shadows, hairlines, badges, text, handles and HUD.
2. **Canvas hit-testing and gesture state** — `zone_at_percent`, `boundary_at`, drag/merge/draw state machines (currently in `canvas.rs` + `state.rs`).
3. **Drag overlay window** — frameless, always-on-top, translucent; mouse-passthrough must be verified; Win32 layered window fallback if Slint cannot do it.
4. **Global hotkeys** — keep `global-hotkey`; Slint's in-app shortcuts are insufficient.
5. **Close-to-tray / `CloseAction`** — Slint close callback + window hide/show semantics on Windows.
6. **App discovery** — move `rail.rs` enumeration (Start Menu `.lnk`, registry `reg query`, `tasklist`) into a pure Rust module; no Slint equivalent.
7. **File picker** — keep `rfd` unless Slint's dialogs prove sufficient.
8. **Undo/redo** — keep `history.rs`; only the toolbar UI changes.
9. **Theme/token port** — shadcn palette and `CanvasTheme` → `theme.slint` tokens; visual parity is manual work.
10. **Licensing UI** — AboutSlint widget or attribution badge.

---

## 9. Final recommendation

**Do not adopt Slint for ZoneFlow at this time.** The migration is technically possible, but it is not justified by the stated goals:

1. **The RAM premise is unproven.** No credible Slint-vs-egui memory comparison exists. The only official figure ("<300 KiB RAM") describes the embedded runtime core, not a desktop app with a renderer. The current app already measures ~59–62 MiB private / ~109–112 MiB working set with a 14.21 MiB exe and <0.5% idle CPU — it is already lightweight.
2. **The likely RAM-saving renderer conflicts with the polish goal.** Slint's software renderer is the smallest but has documented visual limitations (no inner shadows, no rotation/scaling, western-scripts-only text). FemtoVG is GPU-backed like the current glow renderer, so a meaningful RAM reduction is unlikely. Skia is explicitly heavy on disk footprint.
3. **The cost is certain and concentrated in the wrong place.** The app's defining component — the ~1,000-line hand-painted zone canvas — has no Slint equivalent and would need a full redesign via pixel-buffer uploads or backend-specific rendering hooks. That is the most likely place to lose the "modern, polished" quality the migration is supposed to improve.
4. **The backend benefit is zero.** Global hotkeys, tray, Win32 engine, config and app discovery all stay; Slint does not simplify them.
5. **Additional friction:** a new DSL and toolchain to learn, recent desktop maturity (tray/shortcuts/drag-drop only in 1.16–1.19), menu customization limits, Windows build gotchas, and a licensing attribution obligation.

**Recommended alternative path (no toolkit swap):**
- First **profile where the current ~60 MiB private / ~110 MiB working set actually goes** (font atlas, embedded JetBrains Mono, glow/GPU allocations, shadcn theme). There may be low-hanging reductions — e.g. trimming `default_fonts` or the embedded font — at a fraction of a migration's cost.
- Pursue the **existing egui GUI architecture plan** (`docs/plans/zoneflow-gui-architecture.md`) for the "modern, polished" goal; it is already written, approved, and far cheaper than a toolkit swap.
- Correct the stale README figures (~10 MB / ~49 MB / 30 tests) to the measured values.

**When to revisit:** only if the Phase 0 spike (§5) produces a material, repeatable RAM reduction on this machine with acceptable visual parity, and the product owner agrees it justifies the rewrite. Until then, this document is a feasibility record — not an authorization to migrate.
