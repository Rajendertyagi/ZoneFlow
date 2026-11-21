# ZoneFlow Studio (Rust)

A lightweight Windows window-tiling and automation studio — a pure-Rust replacement for the original AutoHotkey script. It ships as a **single ~10 MB `.exe`** with no runtime to install: the UI is built with **eframe/egui 0.37 + the glow renderer**, so no HTML, CSS, JavaScript, or webview is bundled or shipped. The design references live in [`docs/design/`](docs/design/) (`design-mockup.html`, `design-v2.html`, `design-v3.html`); they are never loaded by the app.

> **Architecture direction (2026-11-21):** the shipped app described above is the current, working egui build. The approved next direction is a **headless Rust + Axum background process with an on-demand Svelte editor** served from embedded assets — see [`docs/plans/zoneflow-rust-axum-svelte-architecture.md`](docs/plans/zoneflow-rust-axum-svelte-architecture.md). No source or dependency changes have been made for it yet; egui remains the shipped UI until the migration is implemented. Measured figures in this README (~10 MB, ~49 MB private RAM, 30 tests) are stale — see [`docs/performance/results.md`](docs/performance/results.md) for current measurements (14.21 MiB exe, ~59–62 MiB private, 43 tests).

Your original AHK files are untouched at `E:\Apps\Hotkeys\ScreenLayout.ahk` (and `E:\Apps\Hotkeys\ScreenLayoutGUI\`). Stop the AHK script before running ZoneFlow — it grabs the same global hotkeys. Hotkey registration fails gracefully (and logs a warning) if a binding is already taken.

## Features

- **Three-pane studio** — app library + profiles rail, zone canvas, and zone/automation inspector.
- **Profiles & zones** — named workspaces, each containing zones stored as percentages of the work area. One app per zone.
- **Layout wizard (`＋ Layout`)** — start from a template (Duo / Debug / 3 Columns / Focus / Empty), a custom grid, a single-zone preset, or draw zones on a grid.
- **Draw mode** — drag directly on the canvas to paint new zones, with optional 25/50/75 snap guides.
- **App picker** — assign apps from Start Menu shortcuts, registry Uninstall keys, currently running apps, or an exact `.exe` via **Browse**. Drag an app onto a zone to attach it.
- **Drag-to-snap overlay** — `Ctrl`+drag any window title bar to show translucent zone overlays; release to snap into a zone.
- **Global hotkeys** — 17 keyboard bindings plus the `Ctrl`+drag mouse gesture (see table below). All bindings are stored in config and editable in the Hotkeys view.
- **Undo/redo** — `Ctrl+Z` / `Ctrl+Shift+Z` (or `Ctrl+Y`) over zone and profile edits, with a configurable undo depth.
- **Tray menu** — Settings / Re-tile / Reload / Exit, plus close-to-tray with a "remember my choice" option.
- **Autostart** — optional launch at Windows login, plus optional auto-launch-and-snap of bound apps on startup.
- **Config persistence** — versioned JSON config (schema v1) with empty-by-default starter profiles and automatic migration of older configs.

## Build

Requires a current **stable** Rust toolchain, pinned by `rust-toolchain.toml` (`channel = "stable"`, with `rustfmt` and `clippy`). egui 0.37 needs a recent stable release.

```powershell
cargo build --release
```

The executable is produced at `target\release\screenlayout.exe` (roughly **10 MB**; about **49 MB** private RAM at runtime with the glow renderer). The release profile uses `opt-level = 2`, `lto = true`, and `strip = true`.

For development checks:

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Run

Double-click `target\release\screenlayout.exe`. It opens the studio window; right-click the tray icon for Settings / Re-tile / Reload / Exit. Closing the window offers Minimize to tray / Exit / Cancel (with "remember my choice").

## Config & logs

- **`ScreenLayout.json`** — created next to the `.exe` on first run. Plain JSON, schema version **1**; back it up freely. New installs start with three empty starter profiles (Website / Coding / Trading) and no hardcoded apps, URLs, or titles. Older v0 configs are migrated forward without losing profiles or zones; an unreadable file is renamed to `ScreenLayout.json.bad` rather than wiped.
- **`zoneflow.log`** — written next to the `.exe`. Log level is controlled by the `RUST_LOG` environment variable (default `info`), e.g. `RUST_LOG=debug screenlayout.exe`.

## Hotkeys

These defaults mirror the original AHK script and can be changed in the Hotkeys view.

| Hotkey | Action |
|---|---|
| `Ctrl+Shift+1` … `Ctrl+Shift+9` | Launch/focus app by index (9 bindings) |
| `Ctrl+Shift+0` | Re-tile all apps |
| `Ctrl+Alt+Left` | Move active window to previous zone |
| `Ctrl+Alt+Right` | Move active window to next zone |
| `Ctrl+Alt+Up` | Move active window up a row |
| `Ctrl+Alt+Down` | Move active window down a row |
| `Win+Up` | Maximize active window |
| `Win+Down` | Restore active window to its zone |
| `Ctrl+Alt+R` | Reload script/config |
| `Ctrl` + drag window title bar | Drag-to-snap with zone overlays |

That is **17 keyboard bindings** (9 + 1 + 4 + 2 + 1) plus **1 mouse gesture**. `Win+Up` / `Win+Down` may be reserved by Windows and fail to register; all other bindings work. `Ctrl`+drag is a low-level mouse gesture detected in the frame loop, not a global shortcut.

## Adding apps

1. Click **`＋ Layout`** to create a profile and choose a starting layout (or pick an existing profile).
2. Click an empty zone (or use the app picker) to **Choose an app**. The picker lists installed apps (Start Menu + registry), running apps, and a **Browse** tab for an exact `.exe` path.
3. **Drag an app** from the library onto a zone to attach it, or use Browse to pick the precise executable when a shortcut name is ambiguous.
4. Edit the zone in the inspector (launch mode, URL/tab rules, macro, geometry), then **Save & Apply**.

## Architecture

The crate is a binary (`src/main.rs`, no `src/lib.rs`). Modules:

- **Domain model** — a *Profile* is a named workspace containing *Zones*; each zone is a percentage rectangle with **one app binding** (plus launch mode, URL/tab rules, and macro fields). There is no separate apps list — a zone *is* an app's place on screen. Geometry presets rearrange zones while preserving their bindings.
- **`src/config.rs`** — versioned JSON schema (`schema_version` v1), `#[serde(default)]` on all structs, v0 → v1 migration, atomic writes, and the `Settings` struct holding every tunable. Hotkey defaults live in `default_hotkeys()`.
- **`src/engine_core.rs`** — pure zone math: boundary detection, 25/50/75 snapping, percent↔pixel helpers. No Win32 calls; unit-tested in isolation.
- **Win32 engine** — `src/engine.rs` (window enumeration, snapping, DWM, execute/retile/restore), `src/system.rs` (global hotkeys, tray, actions), `src/overlay.rs` (drag-to-snap overlay), and `src/autostart.rs` (login autostart).
- **`src/editor/`** — the egui UI, split into focused modules: `mod.rs` (the `Editor` and frame loop), `state.rs` (view/state types), `canvas.rs` (hand-painted zone canvas), `rail.rs` (profiles + app library), `inspector.rs` (zone/automation inspector), `views.rs` (Hotkeys / Settings views), `toolbar.rs`, `top_bar.rs`, `action_bar.rs`, `dialogs.rs` (wizard, picker, help, confirmations), `history.rs` (undo/redo), `theme.rs`, and `tokens.rs` (layout/color tokens).

All tunables live in the `Settings` struct in `src/config.rs` — never as literals in code. Key ones: `snap_points`, `grid_cols` / `grid_rows`, `min_zone_pct`, `resize_step_pct`, `boundary_tolerance_pct`, `undo_depth`, `display_debounce_ms`, `overlay_inactive_alpha` / `overlay_active_alpha`, `dwm_snap_retries`, `min_window_px`, `zone_tints`, `auto_launch_on_startup`, `resolutions`, `app_list_cap`, `running_apps_ttl_secs`, `taskbar_h`, and `stage_lower_h`. Add new configurables there.

The **glow** renderer is intentional (`default-features = false, features = ["glow", ...]` in `Cargo.toml`); wgpu was rejected because it used ~323 MB RAM versus ~49 MB with glow. The crate stays on **edition 2021** deliberately: edition 2024 makes `unsafe_op_in_unsafe_fn` deny-by-default, which breaks the Win32 `extern "system"` callbacks in `engine.rs`.

## Testing

**30 tests** pass: **27 inline** `#[cfg(test)]` tests (9 in `config.rs`, 15 in `engine_core.rs`, 1 in `engine.rs`, 2 in `system.rs`) plus **3 integration** tests in `tests/config_file_roundtrip.rs` that verify the on-disk JSON contract. Because this is a binary crate with no `src/lib.rs`, the authoritative unit tests live inline in the source modules; see `tests/README.md` for the rationale.

```powershell
cargo test
```

## Known limitations

- **Primary monitor only** — the work area is the primary display's work area.
- The **running-apps** list is a ~2-second cached snapshot (`running_apps_ttl_secs`), not live.
- Installed-app `.lnk` targets are **guessed from shortcut names**; use the Browse tab for an exact executable path.
- The **Launch** button is a console simulation; real execution is driven by the global hotkeys.
- **Icons are Unicode-free** — the UI uses Lucide icons via `shadcn-egui::icons::lucide` (no Unicode glyphs or bundled SVGs).
- **Display changes** are handled by polling the work area every `display_debounce_ms` (default 200 ms) and re-tiling on change — no Win32 message hook.
- **Path-based window matching** is supported: when a zone has a `path`, an exact full-path match is preferred with an exe-name fallback (`find_window_by_path_min`).
- Installer/signing, release-tag artifact attachment, and i18n label centralization are not yet done.
