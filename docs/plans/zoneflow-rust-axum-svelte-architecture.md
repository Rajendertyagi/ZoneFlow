# ZoneFlow — Rust + Axum + Svelte Architecture

**Status:** Authoritative architecture and migration plan. **No implementation is authorized by this file.**
**Date:** 2026-11-21
**Branch:** Based on `main` at `964aac7` (verified clean, matching `origin/main`).
**Supersedes:** `slint-gui-migration-plan.md` as the active implementation direction (that document is retained as historical research). The four approved planning documents remain authoritative for product behavior: `zoneflow-product-workflow.md` (D1–D6), `zoneflow-feature-preservation.md`, `zoneflow-implementation-roadmap.md`, and `zoneflow-gui-architecture.md` (egui-specific layout guidance, now historical for the web UI).
**Companion:** `zoneflow-rust-axum-svelte-implementation-plan.md` contains the code-level API details and task breakdown derived from official documentation. Read this document for architecture and that one for implementation.

---

## 1. Goal and decisions

Replace the resident egui/eframe GUI with a **headless Rust background process** that serves an on-demand Svelte editor over a local HTTP API. The browser is opened only when the user requests the editor; closing it must not terminate the Rust process. Tray, global hotkeys, Win32 window management, autostart and config compatibility are preserved unchanged.

**Verified motivation:** the headless Rust + Axum spike measured ~2.0–2.1 MiB private / ~10.9–12.5 MiB working set versus ~59–62 MiB private / ~109–112 MiB working set for the current egui app. That is the always-on process only — not the complete application with a browser open (see §9).

**Locked constraints:**
- No Node.js/Bun in the installed app — build-time only.
- No Electron, Tauri, WebView or embedded browser.
- No separate frontend server in production; Rust serves embedded `dist/`.
- No database unless a genuine requirement is demonstrated (none exists).
- Primary monitor only. No multi-monitor.
- All D1–D6 product decisions and feature-preservation requirements remain in force.

---

## 2. Current architecture assessment

The crate is a binary (`src/main.rs`, no `src/lib.rs`). The backend is already toolkit-independent:

| Module | GUI dependency | Migration impact |
|---|---|---|
| `src/engine_core.rs` | None (only `config::{Profile, Zone}`) | Unchanged |
| `src/engine.rs` | None (`windows`, `config`, `engine_core`) | Unchanged |
| `src/config.rs` | None (`serde`, `fs`, `OnceLock`/`RwLock`) | Unchanged |
| `src/system.rs`, `src/autostart.rs` | None (`global-hotkey`, `tray-icon`, Win32) | Unchanged |
| `src/main.rs`, `src/editor/*`, `src/overlay.rs` | `eframe`, `egui`, `shadcn-egui`, `rfd` | Replaced by HTTP API + Svelte UI |

All boundary types are plain owned Rust data with no egui types. **Serde status differs by type:** `AppConfig`, `Settings`, `Profile`, `Zone` and `ResolutionPreset` derive `Serialize`/`Deserialize`; `engine_core::Rect`, `KeyEvent` and `system::Action` do **not**. The HTTP layer therefore needs its own DTOs (or serde derives added to those three types during implementation) — do not assume every backend type can be put directly in a `Json` response. `AppConfig` is already `Send + Sync` and cloned across a worker thread today. The backend can be retained verbatim; the migration surface is the GUI layer plus a new HTTP/API layer.

**Remaining backend work (from feature-preservation, not caused by this migration):** B4 gesture guards (GUI-side), B6 config-source unification, B10 `restore_active` parity, B12 dead `is_auto_start()`. These remain tracked in the roadmap; the new UI must not reintroduce them.

---

## 3. Target architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│  Rust executable (always running)                                    │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ Native thread: System (tray + hotkeys) + Win32 message pump   │  │
│  │ GetMessageW → TranslateMessage → DispatchMessageW             │  │
│  └───────┬──────────────────────────────────────────▲───────────┘  │
│          │ Actions (mpsc)                          │ Menu/hotkey    │
│          │                                         │ updates (mpsc) │
│  ┌───────▼─────────────────────────────────────────┴──────────┐    │
│  │ Command worker thread (synchronous Win32 engine calls)      │    │
│  │ Application state: draft · dirty · applied · enabled · ...   │    │
│  └───────┬──────────────────────────────────────────▲──────────┘    │
│          │ oneshot replies                         │ commands       │
│  ┌───────▼─────────────────────────────────────────┴──────────┐    │
│  │ Tokio runtime (separate thread): Axum HTTP API              │    │
│  │  /api/* commands + state   ·   /api/events (SSE)            │    │
│  │  embedded dist/ static assets (rust-embed)                  │    │
│  └───────┬─────────────────────────────────────────────────────┘    │
└──────────┼───────────────────────────────────────────────────────────┘
           │ loopback HTTP, per-launch token
    ┌──────▼───────┐
    │ Browser      │  opened on demand; closing it does not stop Rust
    │ Svelte 5 SPA │  SVG canvas · shadcn-svelte · Tailwind 4
    └──────────────┘
```

### 3.1 Backend separation

| Concern | Owner | Notes |
|---|---|---|
| Windows application lifecycle | `main.rs` | Starts and joins all threads; owns shutdown coordination. Does **not** own the message pump. |
| Native thread | new `native_loop.rs` | Creates and owns `System` (tray + hotkey managers); runs `GetMessageW`/`TranslateMessage`/`DispatchMessageW`. |
| Window-management engine | `engine.rs`, `engine_core.rs` | Unchanged. Geometry stays in `engine_core`. Executed on the command worker thread. |
| Configuration and profiles | `config.rs` | Unchanged schema v1; atomic writes; migration. |
| Local HTTP API | new `api/` module | Axum routers, extractors, command handlers on the Tokio runtime. Thin — sends commands to the worker. |
| Static asset serving | new `web/` module | Embedded `dist/` via the selected crate (§4). |
| Browser/editor session | new `session` module | Token, launch, SSE subscriber set, cleanup on disconnect. |
| Tray and global hotkeys | `system.rs` | Unchanged; owned by the native thread; emits `Action`s to the command worker. |

The authoritative application state lives in Rust, not the browser. The browser holds a mirrored view for rendering and optimistic previews; every durable mutation goes through an API command.

### 3.2 Thread ownership and communication (authoritative)

`tray-icon` and `global-hotkey` require a real Win32 message loop on the **same thread that creates their managers**. A Tokio timer is not a substitute. The ownership model is therefore fixed:

| Thread | Creates / owns | Runs |
|---|---|---|
| Native thread | `System`, tray icon, hotkey managers, overlay window | `GetMessageW` → `TranslateMessage` → `DispatchMessageW`; calls `System::poll()`; if it blocks on handles, uses `MsgWaitForMultipleObjectsEx` |
| Command worker thread | Authoritative `Inner` state, history, engine calls | Synchronous engine operations (including the 8 s launch wait), serialized |
| Tokio runtime thread(s) | Axum server, SSE subscribers, HTTP handlers | `axum::serve`; handlers send commands and await `oneshot` replies |
| Main thread | Process bootstrap, shutdown coordination | Starts the three threads above and joins them on exit |

**Communication paths (channels only — no event framework):**

1. **Native → worker:** `System::poll()` yields `Action`s (hotkey, tray menu clicks); an `mpsc::Sender<Command>` forwards them to the command worker.
2. **HTTP → worker:** each API handler sends `Command { payload, reply: oneshot::Sender<Result<...>> }` and awaits the reply. Blocking engine work never runs on a Tokio worker.
3. **Worker → HTTP/native:** state changes are broadcast to SSE subscribers; results return via the `oneshot` reply.
4. **Reverse path (API → native) — critical:** profile create/rename/delete/select, hotkey edits, settings changes and Disable/Enable all originate as API commands. After the worker mutates state it sends a `NativeUpdate` over an `mpsc` channel **back to the native thread**, which rebuilds the tray profile submenu, updates the applied-profile checkmark, toggles the Disable/Enable label, and re-registers hotkeys. The native thread never reads shared state directly — it applies updates only from its channel. This keeps tray and hotkeys in sync with API changes without shared mutable GUI state.
5. **Shutdown:** tray "Exit" or a close request posts `WM_QUIT` to the native loop and fires the axum `oneshot` shutdown signal. The main thread then joins native, worker and Tokio threads in that order; each channel closes so receivers terminate. No thread is left detached.

**Native wake-up (required — an `mpsc` send does not wake `GetMessageW`):** a thread blocked in `GetMessageW` stays asleep until a Windows message arrives, so a queued `NativeUpdate` or command result would otherwise sit unprocessed until unrelated input. The native thread therefore owns a thread-safe queue (`Mutex<VecDeque<NativeMessage>>`) for the **payloads**, and every producer that enqueues a message also calls `PostThreadMessageW(native_thread_id, WM_APP, 0, 0)` as a **wake-up signal only**. On `WM_APP` the native loop drains the queue and processes each item (rebuild tray menu, re-register hotkeys, deliver a command result to its waiting `oneshot`). The native thread ID is captured at startup with `GetCurrentThreadId()` and shared with producers as a small `Send`-safe handle. Shutdown still uses `WM_QUIT`.

The native thread **never blocks synchronously** on an engine operation (which can take 8 s): it sends a command to the worker and returns immediately to the message loop; the result arrives later through the queue + `WM_APP` wake-up. This is what makes the D2 `409` confirmation responsive — the native thread is free to show the dialog while the worker checks dirty state, and receives the result via the wake-up.

This design uses only `mpsc`/`oneshot` channels, a small payload queue, and `PostThreadMessageW` — no elaborate event framework.

### 3.3 Tray and profile workflow (D1–D6)

The tray is the primary daily interface and must work with no browser open. `src/system.rs` currently builds a fixed menu; the new design builds it dynamically on the native thread from a `NativeUpdate` (§3.2):

```
ZoneFlow Studio — Applied: <applied> · Selected: <selected>
─────────────────────────────
Profiles              ▸  Website ✓  (checkmark = applied)
                          Coding •   (bullet = selected, when different)
                          Trading
─────────────────────────────
Disable layout           (toggles to Enable layout)
Open editor…
Settings…
Exit
```

- Rebuild the profile submenu after every profile create/rename/delete/select. Show **both** dimensions: a checkmark on the **applied** profile and a distinct marker (bullet) on the **selected** profile when they differ. The header names both. This matters when layout is disabled: selecting a profile changes Selected but not Applied, and the tray must show that without opening the editor.
- **D6:** tray profile click applies immediately when enabled; when disabled it selects only. Home selection and editor-rail loading use their own API operations (`select` / `load`) and remain separate.
- **D2:** a tray switch with a dirty draft shows a native Windows confirmation ("Unsaved changes in <profile>. Discard them and switch?") with Discard/Cancel. Never silently discard. The API returns 409 on dirty; the dialog is the user-facing resolution before retrying with an explicit discard flag.
- **D4:** Disable/Enable suspends all automation; runtime-only; label toggles.
- Open editor / Settings open the browser editor (Task B6/F1); Exit performs graceful shutdown (§3.2).

---

## 4. Dependency choices (verified 2026-11-21)

### 4.1 Frontend (npm registry, current versions)

| Package | Version | License | Notes |
|---|---|---|---|
| `svelte` | 5.59.1 | MIT | Runes (`$state`, `$derived`, `$effect`); compiles to direct DOM updates, no virtual DOM. |
| `vite` | 8.3.8 | MIT | Static `dist/` output; Rolldown bundler; `base: './'` for relative asset URLs. |
| `tailwindcss` | 4.3.5 | MIT | CSS-first `@theme`; automatic content detection/tree-shaking; `@tailwindcss/vite` plugin. |
| `@tailwindcss/vite` | 4.3.5 | MIT | Peer range includes Vite 8. |
| `shadcn-svelte` (CLI) | 1.8.1 | MIT | Copy-paste source components on Bits UI; not a runtime package. |
| `bits-ui` | 2.22.0 | MIT | Headless accessible primitives; Svelte 5. |
| `paneforge` | 1.0.2 | MIT | Resizable panels; **stale (~15 months, not actively maintained)** — see risks. |
| `@lucide/svelte` | 1.77.0 | ISC | Use this package, not legacy `lucide-svelte`; zero runtime deps; tree-shakeable. |
| `@sveltejs/vite-plugin-svelte` | 7.3.1 | MIT | Peers Vite 8 + Svelte ≥5.46.4 — satisfied. |

**Build-time requirement:** Node ≥20.19 (Vite 8, plugin, Bits UI, Tailwind 4 oxide). Node is **not** needed at runtime — `vite build` emits plain static HTML/JS/CSS. No webview or browser runtime is pulled in by any of these packages.

**Compatibility:** all listed versions are mutually compatible. Tailwind 4 raises the effective browser floor to Chrome 111 / Firefox 128 / Safari 16.4 — acceptable for a modern Windows utility. Tailwind 4 is not designed for Sass/Less/Stylus; avoid them.

### 4.2 Rust static-asset serving — decision: `rust-embed` 8.14.0

Verified comparison (crates.io/docs.rs, 2026-11-21):

| | `rust-embed` 8.14.0 | `include_dir` 0.7.4 | `static-serve` 0.6.4 |
|---|---|---|---|
| Embeds at compile time | Yes (release) | Yes (always) | Yes (always) |
| Dev behavior | Reads from disk (or `debug-embed`) | Always embedded | Always embedded |
| Built-in axum handler | No (~15-line handler) | No | Yes (`static_router()`) |
| Extra runtime deps | `rust-embed-utils`, `walkdir`, `sha2` | none | `bytes`, `range-requests`, macro crate |
| MSRV | 1.80 | 1.64 | **1.88** |
| License | MIT | MIT | MIT OR Apache-2.0 |
| Last release | **2026-11-11** | 2024-06-17 (stale ~2 yr) | 2026-09-06 (young, ~31k downloads) |
| Windows evidence | Path normalization + Wine handling in source | docs.rs Windows targets | none found |

**Decision:** `rust-embed = "8.14.0"` with **default features only** (do **not** enable `axum-ex` — it only enables optional deps for the bundled example; there is no built-in axum handler). Rationale: actively maintained, explicit dev-vs-release behavior (debug reads from disk, release embeds — ideal for a desktop dev loop), Windows path normalization, and no HTTP/async runtime deps. `include_dir` is smaller but stalled; `static-serve` is feature-rich but overkill, forces MSRV 1.88 and edition 2024, and pulls extra deps.

Axum 0.8.9 has **no** built-in `ServeFile`/`ServeDir` in `axum::response` (verified across 0.5–0.8); disk serving requires `tower-http` with the `fs` feature. We do not need it — embedded bytes are served directly via `IntoResponse`, as in rust-embed's official axum example. Add an SPA fallback to `index.html`. Use forward slashes in `#[folder = "dist/"]` and lookups.

**Resolved:** `Metadata` in rust-embed 8.14.0 exposes only `sha256_hash()`, `last_modified()` and `created()` — **no `mimetype()` method**. Derive `Content-Type` from the file extension in the handler (a small `match` on the suffix), or add `mime_guess` explicitly. No extra crate is needed for the minimal path.

### 4.3 Rust runtime — verified versions

| Crate | Version | License | Notes |
|---|---|---|---|
| `axum` | 0.8.9 | MIT | MSRV 1.80; built-in `Sse` response type. |
| `tokio` | 1.54.1 | MIT | MSRV 1.85; **1.54.0 was yanked** — use 1.54.1. |
| `rust-embed` | 8.14.0 | MIT | See §4.2. |

Minimal Tokio features for the HTTP server: `["rt-multi-thread", "macros", "net"]`. Windows is a guaranteed supported platform; the `net` feature pulls `windows-sys` with the needed Win32 features. Effective MSRV floor for the new stack is **1.85** (tokio), below the project's current `rust-version = "1.85"`. All recommended crates are permissive (MIT); no copyleft concerns.

Recommended minimal dependency block (resolve the `mimetype()` item above in Phase 1):

```toml
[dependencies]
axum = "0.8.9"
tokio = { version = "1.54.1", features = ["rt-multi-thread", "macros", "net"] }
rust-embed = "8.14.0"   # default features; do NOT enable axum-ex
```

---

## 5. API contracts

All responses are JSON. All requests require the per-launch token (§8). Commands are `POST` and return either a success body or a structured error `{ "error": { "code": string, "message": string } }`.

### 5.1 State and transport

- `GET /api/state` → full snapshot: draft config, dirty flag, applied profile, enabled flag, selected zone, profiles, zones, settings, hotkeys, recent logs, engine status.
- `GET /api/events` → SSE stream of `{type, payload}` events: `state-changed`, `log`, `status`, `hotkey-action`, `apply-result`.
- `GET /api/status` → lightweight status poll (fallback if SSE is unavailable).

**Transport decision:** use **SSE** for backend→UI events and plain `fetch()` for commands. SSE is built into Axum, works with the browser's `EventSource`, needs no extra dependency, and handles the actual requirement (push logs/status/state changes). Native `EventSource` cannot set a bearer header, so it authenticates with the session cookie established by the bootstrap flow (§8); the auth guard applies only to `/api/*`, never to static assets. **WebSocket is unnecessary** — there is no bidirectional streaming need; high-frequency pointer previews stay in the frontend. Polling is a fallback only, not the primary mechanism.

### 5.2 Profiles

| Method | Path | Body | Result |
|---|---|---|---|
| GET | `/api/profiles` | — | `[{name, zone_count, applied, selected}]` |
| POST | `/api/profiles` | `{name}` | created profile |
| PUT | `/api/profiles/{name}` | `{new_name}` | renamed |
| DELETE | `/api/profiles/{name}` | — | deleted; applied dimension cleared if it was applied |
| POST | `/api/profiles/{name}/duplicate` | `{new_name}` | copy |
| POST | `/api/profiles/{name}/load?discard_dirty=false` | — | **D6 editor rail:** load profile into the draft for editing; does not apply. Dirty-guarded (D2). |
| POST | `/api/profiles/{name}/select?discard_dirty=false` | — | **D6 Home:** select only — update the Selected dimension, no draft change, no apply, no launch. Dirty-guarded (D2). |
| POST | `/api/profiles/{name}/apply?discard_dirty=false` | — | **D6 tray:** apply immediately when layout is enabled; when disabled, behave as select-only. Dirty-guarded (D2). |

All three accept an optional `discard_dirty: bool` (default `false`). **D2 dirty-draft contract (unified for tray, Home and rail):**

1. The caller sends the switch command with `discard_dirty=false`. The command worker owns the dirty state and checks it.
2. **Clean draft** → worker executes the switch immediately and returns success.
3. **Dirty draft + `discard_dirty=false`** → worker makes **no state change** and returns `409` with `{ error: { code: "dirty_state", message: "..." } }`. The native thread never reads dirty state directly — it learns it from this reply.
4. **Dirty draft + `discard_dirty=true`** → worker discards the draft (reverts to the saved profile), then executes the switch.

**Confirmation resolution:** the web SPA shows a shadcn Dialog on 409 and retries with `discard_dirty=true` on Discard; Cancel leaves everything unchanged. The tray (no browser) receives the 409 in the native thread, shows a native `MessageBoxW` ("Unsaved changes in <profile>. Discard them and switch?"), and on Discard resends the same command with `discard_dirty=true`; Cancel aborts. This is one flow — worker detects dirty, caller confirms, worker executes or aborts — with no new infrastructure.

These are three distinct operations, not one. Tests must verify each path independently, including the dirty/clean × discard/cancel matrix for all three.

### 5.3 Zones

| Method | Path | Body | Result |
|---|---|---|---|
| GET | `/api/zones` | — | zones of the active draft profile |
| POST | `/api/zones` | `{geometry?}` | new zone |
| PUT | `/api/zones/{id}` | `{label, exe, path, launch_mode, url, title_match, tab_behavior, macro...}` | updated zone |
| DELETE | `/api/zones/{id}` | — | removed (refuse last zone) |
| POST | `/api/zones/{id}/split` | `{axis: "v"\|"h", ratio: f32}` | two zones |
| POST | `/api/zones/merge` | `{source_id, target_id}` | merged zone |
| POST | `/api/zones/{id}/assign` | `{label, exe, path}` | D3 replace-assignment rules apply |
| DELETE | `/api/zones/{id}/assignment` | — | clears zone fields only; library entry untouched |
| POST | `/api/undo`, `/api/redo` | — | history navigation |

### 5.4 Applications and discovery

| Method | Path | Body | Result |
|---|---|---|---|
| GET | `/api/apps?source=installed\|running` | — | cached app entries (2 s TTL for running) |
| POST | `/api/apps/browse` | — | native `.exe` picker result (Rust-side `rfd`, triggered from the UI) |

App discovery stays in Rust (moved out of `rail.rs` into a pure module). The global library remains runtime enumeration, never persisted assignments.

### 5.5 Actions (D1–D6)

| Method | Path | Behavior |
|---|---|---|
| POST | `/api/save` | Persist draft; clear dirty. No window changes. |
| POST | `/api/apply` | **D1:** arrange already-running apps only. **409 if dirty (D2).** Inactive when disabled (D4). |
| POST | `/api/launch-arrange` | **D1:** explicit launch + arrange; respect Launch/Attach; per-zone failures. **409 if dirty.** |
| POST | `/api/preview` | **D5:** inspect steps without running. |
| POST | `/api/revert` | Reload draft from disk; clear dirty; reset selection/transient state. |
| POST | `/api/disable`, `/api/enable` | **D4:** runtime-only flag; suspends all automation. |

### 5.6 Settings, hotkeys, logs

| Method | Path | Result |
|---|---|---|
| GET/PUT | `/api/settings` | `Settings` object; PUT triggers autostart sync where relevant. |
| GET/PUT | `/api/hotkeys` | hotkey map; PUT re-registers global hotkeys. |
| GET | `/api/logs?since=N` | execution log lines (console). |

---

## 6. Frontend component structure

```
src/
  App.svelte                     root shell + view switching
  lib/
    api/client.ts                fetch wrapper (token, error handling)
    api/events.ts                SSE subscription + reconnect
    state/app.svelte.ts          $state mirror of backend snapshot
    components/
      TopBar.svelte              profile, status dimensions, actions
      StatusBar.svelte           engine state, applied/enabled, version
      LeftRail/
        AppLibrary.svelte        searchable, draggable app entries
        ProfilesPanel.svelte     CRUD + select
        ToolsDock.svelte         Canvas / Hotkeys / Settings / Help
      Canvas/
        ZoneCanvas.svelte        SVG root, pointer handling, DnD target
        ZoneRect.svelte          one zone: tint, badge, label, handles
        SnapGuides.svelte        25/50/75 guides + HUD
        DragPreview.svelte       optimistic local preview
      Inspector/
        ZoneInspector.svelte     contextual panel
        ProcessTab.svelte        label/exe/path/launch mode
        BrowserTab.svelte        URL/title/tab behavior
        MacroTab.svelte          trigger/keys/delay
        Console.svelte           log stream
      dialogs/
        WizardDialog.svelte      template/grid/single/draw + live preview
        PickerDialog.svelte      Installed / Running / Browse
        ConfirmDialog.svelte     D2/D3 confirmations
        CloseDialog.svelte       unsaved-changes close behavior
      views/
        HotkeysView.svelte
        SettingsView.svelte
```

Layout: left rail (library + profiles), center SVG canvas (maximum space), right contextual inspector, top profile/status/actions, bottom minimal status. Panels use shadcn-svelte `Resizable` (Paneforge) where resizing is wanted; the inspector is contextual, not permanently forced.

---

## 7. SVG canvas implementation

- **Coordinates:** zones are stored as normalized percentages in `config.rs`. The SVG uses a `viewBox` matching the work-area pixel dimensions from `engine::work_area()`; zone percentages are converted to SVG coordinates in the frontend using that work-area size. Geometry rules (snapping, boundary detection, min size) remain in `engine_core` on the Rust side; the frontend sends normalized values in commands and receives authoritative values back.
- **Rendering:** one `<rect>` per zone with tint fill, accent bar, number badge, label/exe text, and resize handles. Snap guides are `<line>` elements. Selection/drop/merge states are CSS classes.
- **Pointer interaction:** `pointerdown`/`pointermove`/`pointerup` with `setPointerCapture` for zone move, divider resize, draw mode and pick-target merge. Temporary pointer positions and visual previews stay in frontend `$state` — **no HTTP request per pointer move**. Commit a single command on `pointerup`.
- **Drag-and-drop:** app library → zone uses HTML5 drag-and-drop or pointer-based custom DnD; drop-target highlight is local. Assignment is committed via `POST /api/zones/{id}/assign`.
- **Context menu:** shadcn-svelte `ContextMenu` (Bits UI primitive), clamped to viewport.
- **Undo/redo:** Rust owns the history stack; frontend calls `/api/undo` and `/api/redo`. One undo step per focus session, matching the existing behavior.
- **No large canvas framework.** SVG elements plus pointer events are sufficient for 3–10 rectangles. A canvas library would add weight without benefit.

---

## 8. Security (proportionate to a local utility)

This is a locally hosted management interface, not a public service. The user has explicitly accepted the low-risk profile. The following is the minimum necessary; localhost alone is **not** claimed to be complete security.

**Minimum necessary (verified against current guidance):**
- **Loopback-only binding:** bind `127.0.0.1:0`, never `0.0.0.0` or `::`. Use the literal `http://127.0.0.1:<port>` in the launch URL — on modern Windows `localhost` may resolve to `::1` first and stall. Read the assigned port from `listener.local_addr().port()` (the pattern in axum's official SSE example). A random port is a convenience, **not** a security boundary.
- **Per-launch random token + bootstrap session:** generate 32 bytes with a CSPRNG (`getrandom`), URL-safe encoded. Rust opens `http://127.0.0.1:<port>/#token=<token>` — the **fragment** is never sent in HTTP requests, logs or `Referer`. The SPA reads it in JS, calls `POST /api/session` with `Authorization: Bearer <token>`, and on success clears the fragment with `history.replaceState(null, '', location.pathname)`. The server sets an `HttpOnly; SameSite=Strict` session cookie. All later requests (fetch and native `EventSource` with `withCredentials: true`) are same-origin and carry the cookie. The auth guard covers **only `/api/*`** — static assets are public. The token is the CSRF defense; a separate CSRF token is redundant. Failure behavior (missing/invalid token, failed exchange, expired session, SSE reconnect) is specified in the implementation plan §1.1.
- **Host header validation:** reject requests whose `Host` is not exactly `127.0.0.1:<port>`. This is the direct, cheap mitigation for DNS rebinding (a malicious page makes the browser contact loopback; the rebounded request carries the attacker's `Host`). Loopback binding alone does **not** stop this.
- **No CORS headers:** the page and API are same-origin, so CORS is irrelevant. Never set `Access-Control-Allow-Origin: *` — that would let any origin read the API.
- **Safe browser opening:** use the `open` crate (`open::that(url)`) or `webbrowser`, never `cmd /C start` (shell injection). The `open` crate sets `CREATE_NO_WINDOW` on Windows. Keep its `insecure` feature off. Construct the URL from a literal scheme/host, numeric port and URL-safe token — no untrusted input. `--app=` mode (Chromium/Edge only) is a UX convenience, not a security control.
- **Session cleanup:** tie per-session state to the SSE stream lifetime (RAII drop guard) or reap sessions after a short grace period (30–60 s) so `EventSource` auto-reconnects are not treated as logout. The whole server exits with the app, so cleanup can stay simple.
- **Response headers:** `Referrer-Policy: no-referrer` and `X-Content-Type-Options: nosniff`.

**Optional hardening (cheap, defense-in-depth):** validate `Origin`/`Sec-Fetch-Site` on state-changing requests (403 on mismatch — MCP-spec baseline for local HTTP servers); constant-time token comparison; `Cross-Origin-Resource-Policy: same-origin` and a basic CSP (`default-src 'self'`); log rejected requests. The URL fragment for the initial token is **not** optional — it is the chosen bootstrap design above.

**What localhost alone does NOT protect against:** DNS rebinding, cross-origin form POSTs, other local processes, malicious browser extensions, XSS in our own UI, malware already on the machine, or plaintext sniffing by privileged local processes. These are accepted or covered by the controls above; none justify heavy security theater for this utility.

Sources: MCP transport spec (loopback bind, Origin validation), GitHub Security Lab DNS-rebinding article (Host header mitigation), MDN Same-Origin Policy / CORS, OWASP CSRF cheat sheet, `open` crate docs.

---

## 9. Performance strategy

Three workloads must be measured and reported separately (see `docs/performance/results.md` and `benchmark.ps1`):

1. **Background** — Rust backend, tray, hotkeys, HTTP server; browser closed. Measured: ~2.0–2.1 MiB private / ~10.9–12.5 MiB WS for the spike, versus ~59–62 MiB private / ~109–112 MiB WS for the current egui app.
2. **Editor open** — Rust backend plus browser with the full Svelte UI. **Not yet measured** (frontend does not exist). Measure Rust and browser process trees separately.
3. **Pointer activity** — editor open plus 20 s of synthetic cursor movement. **Not yet measured.** This is **not** editing: the cursor does not drag or resize a zone. Use `-Mode PointerActivity` in `benchmark.ps1`. Real SVG zone-boundary dragging is a separate dev-only functional test (§4 of the implementation plan); no resource values are claimed for it.

Metrics: working set, private bytes, CPU (Rust and browser reported separately, including browser CPU for Editor open), cold startup, executable size. Run ≥3 times, report ranges. **Do not present the headless backend's RAM improvement as the memory consumption of the complete application.**

---

## 10. Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Browser-on-demand UX is unusual for a tray utility | High | Accept explicitly; use `--app=` where available; keep tray as primary daily interface. |
| Native drag overlay still needs a Win32 layered window | High | Keep a small native window module; browser cannot do click-through always-on-top. |
| Paneforge 1.0.2 is stale (~15 months) | Medium | Use only shadcn-svelte's `Resizable`; isolate usage so it can be replaced. |
| D1–D6 parity lost in the rewrite | High | API contracts encode the rules; checkpoint tests per decision. |
| Native file picker from a web UI | Medium | Keep `rfd` on the Rust side, triggered via `/api/apps/browse`. |
| SVG canvas performance / coordinate bugs | Medium | Keep geometry in Rust; normalized coordinates; commit on pointerup only. |
| Two runtimes to debug | Medium | Clear API boundary; SSE event log; integration tests on the API. |
| Bundle size growth | Low | Tree-shaken shadcn-svelte/Lucide; measure embedded `dist/` size. |
| Node build requirement | Low | Document Node ≥20.19 as build-only; CI uses it; shipped app does not. |
| Remaining backend bugs (B4/B6/B10/B12) | Medium | Track in roadmap; do not reintroduce in the new UI. |

---

## 11. Migration phases and integration checkpoints

egui dependencies are **not removed until Phase 6**, so the app remains buildable throughout.

### Phase 0 — Foundation and baseline ✅ DONE
- Headless spike built and measured; three-workload benchmark harness and results recorded.
- **Checkpoint 0:** background RAM reduction verified; architecture approved.

### Phase 1 — Backend HTTP API
- Add Axum/Tokio/static-asset crate; build `api/`, `web/`, `session` modules; implement all §5 endpoints against the existing state; keep egui UI running.
- **Checkpoint 1:** `cargo fmt/clippy/test/release` clean; API integration tests pass; egui UI still works.

### Phase 2 — Frontend scaffold and embedded assets
- Create the Svelte/Vite/Tailwind/shadcn-svelte project; build `dist/`; embed it; serve it from Axum; open browser on demand from the tray.
- **Checkpoint 2:** browser loads the editor from the Rust exe with no separate server; closing the browser leaves Rust running; tray/hotkeys still work.

### Phase 3 — SVG canvas parity
- Implement zone rendering, move/resize, split/merge, snap guides, draw mode, DnD assignment, context menu, undo/redo.
- **Checkpoint 3:** all canvas journeys pass manually; geometry matches `engine_core`; no HTTP request per pointer move.

### Phase 4 — Panels, dialogs, inspector, settings/hotkeys
- Port left rail, inspector tabs, wizard, picker, confirmations, close dialog, Hotkeys/Settings views.
- **Checkpoint 4:** full user journey passes; D1–D6 behaviors verified; no data loss.

### Phase 5 — Lifecycle and native integration
- Native thread + message pump owning tray/hotkeys; dynamic profile submenu with applied checkmark; D2 tray Discard/Cancel dialog; reverse `NativeUpdate` path; browser session management; native overlay window; close/minimize-to-tray; graceful shutdown joining all threads; SSE cleanup.
- **Checkpoint 5:** tray-only operation works; menu stays in sync with API changes; dirty-draft tray switch confirms; overlay works; hotkeys fire while unfocused; browser close does not stop Rust; all threads join on exit.

### Phase 6 — Verification, performance, egui removal
- Measure all three workloads; remove egui/eframe/shadcn-egui/rfd-from-UI dependencies only after parity; update README/CHANGELOG/plans.
- **Checkpoint 6:** all automated checks clean; performance recorded with no unexplained regression; docs match the shipped binary.

### Phase 7 — Release hardening
- Installer/signing, CI artifacts, final docs sync.
- **Checkpoint 7:** release produced; all §8 verification items checked.

---

## 12. Verification checklist

- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` clean at every checkpoint.
- [ ] API integration tests cover every endpoint in §5, including D1–D6 error cases (409 on dirty, disabled-mode guards).
- [ ] Browser loads from embedded assets with no Node/Bun and no separate server.
- [ ] Closing the browser does not terminate Rust; tray/hotkeys continue.
- [ ] All canvas interactions in §7 work; geometry matches `engine_core`.
- [ ] Three performance workloads measured and reported separately.
- [ ] No source/dependency changes made by this planning task.
