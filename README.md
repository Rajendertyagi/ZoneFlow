# ZoneFlow Studio (Rust)

A lightweight Windows window-tiling and automation studio — a pure-Rust replacement for the original AutoHotkey script. It ships as a **single ~3.65 MB `.exe`** with no runtime to install: a **headless Rust background process** (Axum HTTP API on loopback) with an **on-demand Svelte 5 editor** served from embedded assets. There is no Electron, Tauri, WebView or embedded browser in the installed app — the browser opens only when you ask for the editor, and closing it never stops Rust. The design references live in [`docs/design/`](docs/design/) (`design-mockup.html`, `design-v2.html`, `design-v3.html`); they are never loaded by the app.

> **Architecture (implemented, 2026-11-22):** the headless Rust + Axum + Svelte direction described in [`docs/plans/zoneflow-rust-axum-svelte-architecture.md`](docs/plans/zoneflow-rust-axum-svelte-architecture.md) is now the shipped implementation (Phases 1–6 complete). The former egui/eframe UI and overlay were removed in Phase 6. Measured figures and test counts in this README are the actual Phase 6 values; see [`docs/performance/results.md`](docs/performance/results.md) for the measurement record.

Your original AHK files are untouched at `E:\Apps\Hotkeys\ScreenLayout.ahk` (and `E:\Apps\Hotkeys\ScreenLayoutGUI\`). Stop the AHK script before running ZoneFlow — it grabs the same global hotkeys. Hotkey registration fails gracefully (and logs a warning) if a binding is already taken.

## Features

- **Always-on background process** — tray, global hotkeys, window management and a loopback HTTP server run with no GUI toolkit resident. Measured background footprint: **~2 MiB private / ~12 MiB working set**, 0.00–0.10% CPU.
- **On-demand browser editor** — a Svelte 5 single-page app (SVG canvas, shadcn-svelte, Tailwind 4) served from assets embedded in the `.exe`. Open it from the tray; closing the browser leaves Rust running.
- **Three-pane studio** — app library + profiles rail, zone canvas, and zone/automation inspector.
- **Profiles & zones** — named workspaces, each containing zones stored as percentages of the work area. One app per zone.
- **Layout wizard (`＋ Layout`)** — start from a template (Duo / Debug / 3 Columns / Focus / Empty), a custom grid, a single-zone preset, or draw zones on a grid.
- **Draw mode** — drag directly on the canvas to paint new zones, with optional 25/50/75 snap guides.
- **App picker** — assign apps from Start Menu shortcuts, registry Uninstall keys, currently running apps, or an exact `.exe` via **Browse** (native picker, Rust-side). Drag an app onto a zone to attach it.
- **Drag-to-snap overlay** — `Ctrl`+drag any window title bar to show translucent zone overlays; release to snap into a zone. The overlay is a native layered window owned by the main thread.
- **Global hotkeys** — 17 keyboard bindings plus the `Ctrl`+drag mouse gesture (see table below). All bindings are stored in config and editable in the Hotkeys view.
- **Undo/redo** — `Ctrl+Z` / `Ctrl+Shift+Z` (or `Ctrl+Y`) over zone and profile edits, with a configurable undo depth.
- **Tray menu** — dynamic profile list with applied/selected markers, Disable/Enable, Open editor, Copy editor URL, Settings, Re-tile, Reload and Exit.
- **Autostart** — optional launch at Windows login, plus optional auto-launch-and-snap of bound apps on startup.
- **Config persistence** — versioned JSON config (schema v1) with empty-by-default starter profiles and automatic migration of older configs.

## Build

Requires a current **stable** Rust toolchain, pinned by `rust-toolchain.toml` (`channel = "stable"`, with `rustfmt` and `clippy`). The frontend is built with **Bun** (build-time only — no Node/Bun is needed to run the app).

```powershell
# Frontend (build-time only) — emits frontend/dist/
cd frontend
bun install
bun run build
cd ..

# Mirror the fresh frontend build into the ROOT dist/ that rust-embed embeds.
# Clear it first so stale hashed assets from an older build do not linger.
Remove-Item -Recurse -Force dist
New-Item -ItemType Directory -Force -Path dist | Out-Null
Copy-Item -Recurse -Force frontend/dist/* dist/

# Rust release binary — rust-embed bakes dist/ into the exe at compile time,
# so this MUST run after the frontend build + copy above.
cargo build --release
```

`rust-embed` reads `dist/` at the repository root (`#[folder = "dist/"]` in `src/web/assets.rs`), not `frontend/dist/`. The copy step is required for both local and CI builds; CI performs the same sequence.

The executable is produced at `target\release\screenlayout.exe` (**3,827,200 bytes / 3.65 MiB** in the measured Phase 6 build). The release profile uses `opt-level = 2`, `lto = true`, and `strip = true`. The crate stays on **edition 2021** deliberately: edition 2024 makes `unsafe_op_in_unsafe_fn` deny-by-default, which breaks the Win32 `extern "system"` callbacks in `engine.rs` and `native_loop.rs`.

For development checks:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
# frontend (build + copy dist/ before the release build — see above)
cd frontend; bun run check; bun test; bun run build; cd ..
Remove-Item -Recurse -Force dist
New-Item -ItemType Directory -Force -Path dist | Out-Null
Copy-Item -Recurse -Force frontend/dist/* dist/
cargo build --release
```

## Run

Double-click `target\release\screenlayout.exe`. **No window opens** — the app lives in the system tray. Right-click the tray icon to **Open editor…** (launches the browser editor), **Copy editor URL** (copies the fixed URL to the clipboard), open Settings, Re-tile, Reload config, Disable/Enable layout, or Exit. Closing the browser editor does not stop Rust; use the tray Exit or `POST /api/shutdown` to quit.

The editor is always at **`http://127.0.0.1:4000/`** — a fixed loopback address with no token, login or session cookie. The SPA loads directly: it fetches state and opens the SSE stream on startup. The HTTP server binds only `127.0.0.1:4000`; if that port is occupied, startup logs a clear error and exits rather than falling back to a random port. The `/api` endpoints are open (no authentication or Host validation) — loopback binding is the only boundary.

## Config & logs

- **`ScreenLayout.json`** — created next to the `.exe` on first run. Plain JSON, schema version **1**; back it up freely. New installs start with three empty starter profiles (Website / Coding / Trading) and no hardcoded apps, URLs, or titles. Older v0 configs are migrated forward without losing profiles or zones; an unreadable file is renamed to `ScreenLayout.json.bad` rather than wiped. The on-disk schema is unchanged by the migration — the config round-trip integration tests verify no data loss.
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

That is **17 keyboard bindings** (9 + 1 + 4 + 2 + 1) plus **1 mouse gesture**. `Win+Up` / `Win+Down` may be reserved by Windows and fail to register; all other bindings work. `Ctrl`+drag is detected globally with `GetAsyncKeyState` (150 ms idle poll → 16 ms active tracking), not as a global shortcut.

## Adding apps

1. Open the editor from the tray and click **`＋ Layout`** to create a profile and choose a starting layout (or pick an existing profile).
2. Click an empty zone (or use the app picker) to **Choose an app**. The picker lists installed apps (Start Menu + registry), running apps, and a **Browse** tab for an exact `.exe` path.
3. **Drag an app** from the library onto a zone to attach it, or use Browse to pick the precise executable when a shortcut name is ambiguous.
4. Edit the zone in the inspector (launch mode, URL/tab rules, macro, geometry), then **Save & Apply**.

## Architecture

The crate is a binary (`src/main.rs`, no `src/lib.rs`). It runs **three long-lived threads**:

1. **Main / native** — owns the tray, global hotkeys, the hidden message-only window and the layered drag overlay; runs the `MsgWaitForMultipleObjectsEx` message loop (150 ms idle gesture poll → 16 ms active tracking).
2. **Command worker** — authoritative state, history and synchronous engine calls; receives commands over an mpsc channel and broadcasts SSE events.
3. **Tokio (current_thread)** — Axum HTTP API (`/api/*`, SSE `/api/events`) and embedded static assets on the fixed `127.0.0.1:4000`.

Modules:

- **`src/config.rs`** — versioned JSON schema (`schema_version` v1), `#[serde(default)]` on all structs, v0 → v1 migration, atomic writes, and the `Settings` struct holding every tunable. Hotkey defaults live in `default_hotkeys()`.
- **`src/engine_core.rs`** — pure zone math: boundary detection, 25/50/75 snapping, percent↔pixel helpers. No Win32 calls; unit-tested in isolation.
- **`src/engine.rs`** — Win32 window enumeration, snapping, DWM, execute/retile/restore.
- **`src/system.rs`** — dynamic tray menu, global hotkeys, actions and the D2 native confirmation dialog.
- **`src/native_loop.rs`** — native message loop, payload queue, safety timer, manual-reset shutdown event, ordered shutdown sequence and the layered drag overlay.
- **`src/api/`** — HTTP router (open `/api/*`), DTOs, SSE events, authoritative state and the command worker.
- **`src/web/`** — `rust-embed` static asset handler with SPA fallback.
- **`src/browser.rs`** — fixed editor URL/port constants and the browser-opening helper.
- **`src/clipboard.rs`** — Win32 `CF_UNICODETEXT` clipboard helper for the tray "Copy editor URL" action.
- **`src/apps.rs`** — pure-Rust runtime app discovery for `/api/apps` and `/api/apps/browse`.
- **`src/autostart.rs`** — login autostart.
- **`frontend/`** — Svelte 5 SPA (Vite 8, Tailwind 4, shadcn-svelte, Bits UI, Paneforge, Lucide icons). Built to `frontend/dist/` at build time and embedded into the `.exe`; no separate server and no Node/Bun at runtime.

All tunables live in the `Settings` struct in `src/config.rs` — never as literals in code. Key ones: `snap_points`, `grid_cols` / `grid_rows`, `min_zone_pct`, `resize_step_pct`, `boundary_tolerance_pct`, `undo_depth`, `display_debounce_ms`, `overlay_inactive_alpha` / `overlay_active_alpha`, `dwm_snap_retries`, `min_window_px`, `zone_tints`, `auto_launch_on_startup`, `resolutions`, `app_list_cap`, `running_apps_ttl_secs`, `taskbar_h`, and `stage_lower_h`. Add new configurables there.

## Testing

**126 Rust tests pass** — 123 inline `#[cfg(test)]` unit tests (covering config, engine core, engine, system, apps, browser, clipboard, web assets, native loop and the full API/D1–D6 command matrix) plus 3 integration tests in `tests/config_file_roundtrip.rs` that verify the on-disk JSON contract. Because this is a binary crate with no `src/lib.rs`, the authoritative unit tests live inline in the source modules; see `tests/README.md` for the rationale. The **frontend adds 43 tests** (`bun test`) and `bun run check` (svelte-check + tsc) is clean.

```powershell
cargo test
cd frontend; bun test
```

## Known limitations

- **Primary monitor only** — the work area is the primary display's work area.
- **Browser-on-demand UX** — the editor opens in the default browser rather than a native window; the tray is the primary interface.
- The **running-apps** list is a ~2-second cached snapshot (`running_apps_ttl_secs`), not live.
- Installed-app `.lnk` targets are **guessed from shortcut names**; use the Browse tab for an exact executable path.
- **Same-executable multi-window** limitation remains documented; no multi-window claim.
- **Display changes** are handled by polling the work area every `display_debounce_ms` (default 200 ms) and re-tiling on change — no Win32 message hook.
- **Path-based window matching** is supported: when a zone has a `path`, an exact full-path match is preferred with an exe-name fallback (`find_window_by_path_min`).
- Installer/signing, release-tag artifact attachment, and i18n label centralization remain out of scope. They are not part of the completed Phase 7 release-hardening gate, which covers the CI-produced release artifact (fresh embedded assets) and the verification checklist.
