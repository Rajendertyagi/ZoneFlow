# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). The on-disk config `schema_version` (currently **1**) tracks breaking config changes.

## [Unreleased]

### Stack decision

- Rebuilt the app as **pure Rust with eframe/egui 0.37 + the glow renderer**. No HTML, CSS, JavaScript, webview, Tauri, or GPUI in the shipped application. The `design-*.html` files in `docs/design/` remain design references only. glow was chosen over wgpu for memory (~49 MB vs ~323 MB private RAM).

### Added

- **Config foundation** (`src/config.rs`): versioned JSON schema (`schema_version` v1) with `#[serde(default)]` on all structs, v0 → v1 migration that preserves user profiles/zones, and atomic writes (temp file → remove → rename). Unreadable configs are renamed to `.json.bad` instead of being wiped. New installs start with three empty starter profiles (Website / Coding / Trading) and no hardcoded apps, URLs, or titles. Panic-free `active()` / `active_mut()` helpers.
- **All tunables in a `Settings` struct**: snap points, grid columns/rows, min zone %, resize step, boundary tolerance, undo depth, display debounce, overlay alphas, DWM snap retries, min window size, and zone tints — every value a config default, never a code literal. Hotkey defaults live in `default_hotkeys()`.
- **Pure engine core** (`src/engine_core.rs`): zone math, boundary detection, 25/50/75 snapping, and percent↔pixel helpers extracted from Win32 code, with unit tests.
- **Full egui UI rebuild** (`src/editor.rs`): three-pane studio (app library + profiles rail, zone canvas, zone/automation inspector), custom-painted canvas with shared-boundary resize and snap guides, layout wizard with live preview, zone inspector, undo/redo with editor shortcuts, app picker (Installed / Running / Browse), and a tools dock (Canvas / Hotkeys / Settings / Reset Demo / Help).
- **System integration**: global hotkeys loaded from config (17 keyboard bindings, `Win` mapped to `Meta`), autostart driven by config, and configurable overlay tints/alphas.
- **Logging**: `tracing` subscriber writing `zoneflow.log` next to the exe, level controlled by `RUST_LOG` (default `info`); no `println!` debugging in shipped code.
- **CI**: GitHub Actions workflow (`.github/workflows/ci.yml`) running fmt, clippy (`-D warnings`), tests, and a release build on Windows, uploading the release `.exe` as an artifact.
- **Auto-launch on startup** (`auto_launch_on_startup`, default off): when enabled, tiles the active profile's apps on first frame via a worker thread so window waits never block the UI; results stream to the console.
- **Display-change re-tiling**: polls the work area every `display_debounce_ms` and re-tiles on change (no Win32 message hook).
- **Path-based window matching**: `find_window_by_path_min` prefers an exact full-path match with an exe-name fallback; used by execute/retile/restore.
- **Config-driven engine**: `dwm_snap_retries` and `min_window_px` now flow from `Settings` into window snapping and lookup.

### Changed

- **Engine hardening**: documented all `unsafe` blocks with `// SAFETY:` comments, replaced swallowed results with `tracing` warnings, and removed `unwrap()`/`expect()` from library code.
- **Close / shutdown fix**: resolved an infinite close-dialog loop with an `exiting` flag (dialog, tray, and remembered Exit) and added `eframe::App::on_exit` to save dirty drafts and log on shutdown. Tray/hotkeys are released when `System` drops.
- **Documentation**: rewrote `README.md` to match reality (correct ~10 MB exe size and stable toolchain, config/log locations, hotkeys, app workflow, architecture, and known limitations).

### Removed

- **AHK script detection**: removed the in-app `ahk_script_running()` check and startup warning per user decision. The app has no AHK dependency; hotkey registration fails gracefully if a binding is already grabbed.
- **Dead dependency**: removed the unused `egui_tiles` crate.

### Known limitations

- Running-apps list is a ~2-second cached snapshot; installed-app `.lnk` targets are guessed from shortcut names (Browse gives the exact exe).
- The Launch button is a console simulation; real execution is via hotkeys.
- Primary monitor only; icons are Unicode-free Lucide icons via `shadcn-egui::icons::lucide`.
- Installer/signing, release-tag artifact attachment, and i18n label centralization are not yet done.

### Standardization (Workstreams A–C)

No behavior changes in this pass — these are structural, documentation, and CI improvements only. The suite remains at **30 passing tests** (27 inline + 3 integration).

- **Workstream A — modularization:** split the monolithic `src/editor.rs` into a focused `src/editor/` module tree (`mod`, `state`, `canvas`, `rail`, `inspector`, `views`, `toolbar`, `top_bar`, `action_bar`, `dialogs`, `history`, `theme`, `tokens`). Pure zone math stays isolated in `src/engine_core.rs`; Win32 integration remains in `engine.rs` / `system.rs` / `overlay.rs` / `autostart.rs`.
- **Workstream B — de-hardcoding:** every tunable now lives in the `Settings` struct in `src/config.rs` (snap points, grid size, min zone %, resize step, boundary tolerance, undo depth, display debounce, overlay alphas, DWM retries, min window size, zone tints, resolution presets, app-list cap, running-app TTL, taskbar/stage heights, auto-launch). Hotkey defaults live in `default_hotkeys()`; no magic literals remain in the UI or engine paths.
- **Workstream C — standardization:** refreshed `README.md` to match the final state (glow renderer, stable toolchain, ~10 MB exe, config/log locations, 17 hotkeys + `Ctrl`+drag, architecture, tunables, 30-test suite, known limitations, edition 2021 rationale). Added an advisory `cargo audit` step to CI (`continue-on-error: true` so transitive advisories don't block day-one builds). Moved the three `design-*.html` references into `docs/design/`. Documented the Lucide (Unicode-free) icon set and the deliberate edition 2021 choice.
