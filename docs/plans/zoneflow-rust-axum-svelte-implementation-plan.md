# ZoneFlow — Rust + Axum + Svelte: Detailed Implementation Plan

**Status:** Implementation-ready companion to the architecture plan — **IMPLEMENTED**. Phases 1–6 are complete on `main` (Phase 6 verification/removal at `721638f`); Phase 7 (release hardening) is pending. The code-level details, task breakdown and test classification below are unchanged and were realized by that implementation.
**Finalized:** 2026-11-22 · **Plan commit:** `e1773cd` (this finalization) · **Source baseline:** `5e74e85` (pre-finalization state; the historical test figures below were executed there).
**Relationship:** Implements `zoneflow-rust-axum-svelte-architecture.md`, which is the **single authoritative source** for architecture decisions (thread ownership, startup/shutdown, message mechanism, API contracts, D1–D6). This document contains code-level API details, task breakdown, test classification and acceptance criteria. It must not restate architecture decisions differently — if they disagree, the architecture document wins.

---

## 1. Verified API reference

All patterns below are compositions of APIs verified from official docs on 2026-11-22 (sources in architecture §14). Compositions are marked where they are not verbatim examples.

### 1.1 Axum 0.8.9

```toml
[dependencies]
axum = { version = "0.8.9", default-features = false, features = ["http1", "json", "query", "tokio"] }
tokio = { version = "1.54.1", features = ["rt", "macros", "net", "sync", "time"] }  # current_thread
tokio-stream = { version = "0.1.19", features = ["sync"] }   # BroadcastStream
futures-util = "0.3.34"                                      # only if used directly
serde = { version = "1", features = ["derive"] }             # already present
serde_json = "1"                                             # already present
getrandom = "0.3.4"                                          # token (MSRV 1.63)
rust-embed = "8.14.0"                                        # default features; no axum-ex
open = "5.4.5"                                               # no insecure feature

[dev-dependencies]
tower = { version = "0.5.3", features = ["util"] }           # ServiceExt::oneshot
http-body-util = "0.1.5"                                     # BodyExt::collect
```

**Router/State:** `Router<S>` means "missing state S"; only `Router<()>` can be served. Call `.with_state()` at the top level. `State` must precede body extractors. State is cloned per request → wrap in `Arc`. Never hold a `std::sync` guard across `.await`.

**JSON/Path/Query:** `Json<T>` must be the last extractor; route syntax is `{id}` / `{*rest}` (colon syntax panics). One `Path<T>` per handler. `Query<T>` parse failure → 400.

**Error type** (matches architecture §5 contract):

```rust
#[derive(Debug)]
enum ApiError { DirtyState, Unauthorized, Forbidden, NotFound(String), Internal(String) }
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, msg) = match self {
            ApiError::DirtyState  => (StatusCode::CONFLICT, "dirty_state", "save or discard changes first".into()),
            ApiError::Unauthorized=> (StatusCode::UNAUTHORIZED, "unauthorized", "missing token".into()),
            ApiError::Forbidden   => (StatusCode::FORBIDDEN, "forbidden", "invalid token or host".into()),
            ApiError::NotFound(w) => (StatusCode::NOT_FOUND, "not_found", format!("{w} not found")),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", m),
        };
        (status, Json(json!({ "error": { "code": code, "message": msg } }))).into_response()
    }
}
```

**Auth/Host guard:** `middleware::from_fn_with_state` + `route_layer` on the `/api` sub-router only (added after routes). Accept bearer header (bootstrap/tests) **or** `zf_session` cookie (fetch/EventSource). Validate `Host` equals `127.0.0.1:<port>`. Static assets are public via `.fallback(static_file)`.

**Bootstrap sequence and failure behavior:**

1. Rust generates 32 bytes with `getrandom::fill`, URL-safe encodes, opens `http://127.0.0.1:<port>/#token=<token>` (fragment, never query).
2. SPA reads `location.hash`, calls `POST /api/session` with `Authorization: Bearer <token>`; server sets `HttpOnly; SameSite=Strict` cookie.
3. On success SPA calls `history.replaceState(null, '', location.pathname)` to clear the fragment.
4. fetch and `EventSource('/api/events', { withCredentials: true })` use the cookie.

| Condition | Behavior |
|---|---|
| No fragment / missing token | Error page: "Open the editor from the ZoneFlow tray menu." No `/api/session` call. |
| Invalid token | 403; "Session rejected — reopen from the tray." No retry loop. |
| Failed exchange (network/server) | Retry button re-reads hash and retries once; then instruct reopen. |
| Expired/invalid session later | 401; session-expired banner with "Reopen from tray"; do not clear draft. |
| SSE disconnect | `EventSource` reconnects; cookie authenticates each reconnect; after repeated failures show "connection lost", keep local state. |

Token is per-launch; "expired" means the process restarted (old cookie no longer matches).

**SSE:** `Sse::new(BroadcastStream::new(rx).filter_map(...)).keep_alive(KeepAlive::default())` (15 s default). One `EventSource` per client. `tokio-stream` needs feature `sync`. Each handler must end its stream on shutdown so graceful shutdown cannot hang:

```rust
use futures_util::stream::StreamExt;
use tokio::sync::watch;
async fn events(State(mut stop): State<watch::Receiver<bool>>, State(tx): State<broadcast::Sender<String>>) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let rx = tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|r| async move { r.ok().map(|s| Event::default().data(s)) });
    let stopper = async move { let _ = stop.changed().await; }; // Err (sender dropped) also ends it
    Sse::new(stream.take_until(stopper)).keep_alive(KeepAlive::default())
}
```

`take_until` is `futures_util::StreamExt` (no extra feature); `watch` needs tokio `sync`. The handler holds only a shutdown receiver and a broadcast receiver — it never awaits the command worker, so closing SSE cannot deadlock shutdown.

**Static assets:** `#[derive(Embed)] #[folder = "dist/"] struct Assets;` — `Assets::get(path)` returns `EmbeddedFile { data: Cow<'static,[u8]>, metadata }`. `Metadata` has **no `mimetype()`** — derive Content-Type from the suffix. SPA fallback to `index.html` for extensionless paths; 404 for missing assets. Debug builds read from disk; release embeds.

**Tokio runtime construction (verified):** build the `current_thread` runtime explicitly with both drivers enabled, on the dedicated server thread. `Handle::block_on` cannot drive I/O/timers on a current-thread runtime — only `Runtime::block_on` can.

```rust
use tokio::runtime::Builder;
let server = std::thread::spawn(move || {
    let rt = Builder::new_current_thread().enable_all().build().unwrap(); // enable_all = enable_io + enable_time
    rt.block_on(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();   // read BEFORE serve moves the listener
        bound_tx.send(Bound { port, token }).unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(async { let _ = axum_shutdown_rx.await; })
            .await.unwrap();
    });
});
```

The declared features `["rt","macros","net","sync","time"]` support every API used: `enable_io()` (via `net`), `enable_time()` (via `time`), `spawn_blocking` (`rt`), `TcpListener` (`net`), `watch`/`broadcast`/`oneshot` (`sync`), SSE keep-alive (`time`). `io-util` and `signal` are **not** needed.

**Graceful shutdown (verified from axum 0.8.9 source):** `with_graceful_shutdown` stops accepting connections, then awaits `close_tx.closed()` — i.e. **all** connection tasks. An endless SSE stream hangs that await forever; there is no built-in timeout. Therefore each SSE handler selects on a `watch` shutdown receiver via `StreamExt::take_until`, and shutdown fires `sse_tx.send_replace(true)` **before** the Axum signal (architecture §3.6 steps 2–3). `axum::serve` drops the listener itself; you cannot drop it externally (it is moved into `Serve`).

**Handler tests:** `Router` implements `tower::Service`; use `ServiceExt::oneshot` + `http_body_util::BodyExt::collect`. No server needed.

### 1.2 Win32 native thread (verified windows-rs 0.62 APIs)

Cargo features to add to the existing `windows` dependency: `Win32_System_LibraryLoader` (for `GetModuleHandleW`). `HWND` wraps `*mut c_void` and is `!Send`/`!Sync`; `WPARAM`/`LPARAM`/`LRESULT` are `Send + Sync`.

```rust
use windows::{core::{w, PCWSTR}, Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
}};

const CLASS: PCWSTR = w!("ZoneFlow.MsgHost.7f3a9b7c");
const WM_WAKE: u32 = WM_APP + 1;
const WM_STOP: u32 = WM_APP + 2;
const TIMER_SAFETY_DRAIN: usize = 1;   // always-on 100 ms queue safety net
const IDLE_MS: u32 = 150;              // ~6-7 wakeups/s gesture detection
const ACTIVE_MS: u32 = 16;             // ~60 Hz drag tracking

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_WAKE => { drain_queue(); LRESULT(0) }              // non-blocking only
        WM_TIMER if wp.0 == TIMER_SAFETY_DRAIN => { drain_queue(); LRESULT(0) }
        WM_STOP => { PostQuitMessage(0); LRESULT(0) }         // owner thread; NO DestroyWindow here
        WM_DESTROY => { let _ = KillTimer(Some(hwnd), TIMER_SAFETY_DRAIN); LRESULT(0) }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
```

`WM_STOP` only calls `PostQuitMessage(0)` on the owning thread — it does **not** destroy the window. `WM_QUIT` is never dispatched as a normal window message. Window destruction happens last in the architecture §3.6 sequence, after all producers are joined.

- `RegisterClassW` returns 0 on failure; tolerate `ERROR_CLASS_ALREADY_EXISTS` (1410); register once.
- `CreateWindowExW(WINDOW_EX_STYLE(0), CLASS, w!(""), WINDOW_STYLE(0), 0,0,0,0, Some(HWND_MESSAGE), None, Some(hinstance.into()), None)`.
- `PostMessageW(Some(hwnd), WM_WAKE, WPARAM(0), LPARAM(0))` — check `Result`; on failure log and rely on the safety timer. Queue limit is 10,000; unreachable here but checked.
- `SetTimer(hidden_hwnd, TIMER_SAFETY_DRAIN, 100, None)` → `WM_TIMER` drains the queue as a safety net so no payload is stranded. `WM_TIMER` is low-priority/coalesced (fine for a safety net) and survives modal loops. `KillTimer` does not remove already-queued `WM_TIMER`s, so the WndProc branch is idempotent.
- **Gesture detection + drag polling (idle → active):** the message-only window and pass-through overlay cannot see the initial Ctrl+click over another app, so use global `GetAsyncKeyState` (state at call time; test `0x8000` high bit; `VIRTUAL_KEY` is a `u16` newtype → `vk.0 as i32`; ignore the low bit). The `MsgWaitForMultipleObjectsEx` timeout varies: `IDLE_MS = 150` when idle, `ACTIVE_MS = 16` when a drag is active. Check `gesture_down()` on **every** wake: idle press → show overlay + switch to 16 ms; release → hide + back to 150 ms; active tick → `GetCursorPos` + `UpdateLayeredWindow`. `INFINITE` is not used (could never detect the start). Idle CPU is **not zero** — ~6–7 wakeups/s, ~0.01–0.1% of a core, negligible and far below the old 60 Hz egui loop. No `WH_MOUSE_LL` hook (fragile, system-wide, would not remove these calls) and no raw input.
- **Shutdown event (guaranteed wake-up):** create a manual-reset event `CreateEventW(None, true, false, None)` (feature `Win32_System_Threading`). Include it as handle index 0 in `MsgWaitForMultipleObjectsEx(Some(&[event]), timeout, QS_ALLINPUT, MWMO_INPUTAVAILABLE)`: `WAIT_OBJECT_0` = shutdown, `WAIT_OBJECT_0 + 1` = input, `WAIT_TIMEOUT` = poll tick. `SetEvent(event)` is cross-thread, idempotent, and not subject to the 10,000-message queue limit — the guaranteed path. `PostMessageW(WM_STOP)` remains best-effort for the modal `MessageBoxW` case (its modal loop dispatches window messages but not kernel events; neither makes the dialog return early — both signals are latched and observed after dismissal). Leave the event signaled (no `ResetEvent`); `CloseHandle` on main only after the wait loop exits and all producers are joined. Share via `SharedEvent(HANDLE)` with `unsafe impl Send/Sync` (maintainer-endorsed; `HANDLE` is `!Send`/`!Sync`).
- `GetMessageW(&mut msg, None, 0, 0)` — never pass the hidden HWND as filter; check `-1 / 0 / > 0`. (When using the `MsgWaitForMultipleObjectsEx` loop, drain with `PeekMessageW(..., PM_REMOVE)` instead; both patterns are in architecture §3.7.)
- `HwndSend(*mut c_void)` newtype with `unsafe impl Send/Sync`, justified by an `Arc<AtomicBool>` accepting flag and join-before-destroy ordering.
- D2 `MessageBoxW` owner = `None` (not `HWND_MESSAGE`); its modal loop dispatches window messages so `WM_WAKE`/`WM_TIMER` still arrive.
- **Teardown is the single ordered sequence in architecture §3.6** — do not destroy the window here. `SetEvent` (guaranteed) + `PostMessageW(WM_STOP)` (best-effort) → loop exits on `WAIT_OBJECT_0` or `WM_QUIT` → close SSE → fire Axum shutdown → join Tokio → drop `cmd_tx` → join worker → final drain → kill timer → `DestroyWindow(overlay)` → `DestroyWindow(hidden)` → drop `System` on main → `CloseHandle(event)` last → `UnregisterClassW` (optional).

### 1.3 Tray and hotkeys (verified tray-icon 0.26 / global-hotkey 0.10 / muda 0.21)

- `TrayIcon`, `TrayIconBuilder`, `Menu`, `MenuItem`, `CheckMenuItem`, `Submenu` are all `!Send`/`!Sync` — create, mutate and drop on main. `GlobalHotKeyManager` is `Send + Sync` but create it on main for simplicity.
- `tray.set_menu(Option<Box<dyn ContextMenu>>)` exists (returns `()`); prefer in-place mutation: retain `CheckMenuItem` handles and call `set_checked(bool)`, `MenuItem::set_text`, `Submenu::append/remove`.
- Use `MenuItem::with_id(MenuId::new("profile:<name>"), ...)`; match `ev.id.0` by prefix, never by label.
- **Never call `set_event_handler`** — it is a one-way `OnceCell` switch that permanently disables `receiver()`. Use `MenuEvent::receiver()` / `GlobalHotKeyEvent::receiver()` with a single consumer on main.
- `register_all` is fail-fast; keep the existing per-item loop in `register_from_config` with `failures` tracking.
- `Drop for TrayIcon` calls `DestroyWindow` on the creating thread — let main's function return naturally so objects drop there.

### 1.4 Frontend (verified npm registry 2026-11-22)

| Package | Version | Note |
|---|---|---|
| svelte | 5.59.1 | Runes; no virtual DOM |
| vite | 8.3.8 | `base: './'`; static `dist/`; Rolldown/Oxc |
| @sveltejs/vite-plugin-svelte | 7.3.1 | peers Vite 8 + Svelte ≥5.46.4 |
| tailwindcss / @tailwindcss/vite | 4.3.5 | CSS-first `@theme`; Vite 8 compatible |
| shadcn-svelte | 1.8.1 | copy-paste source on Bits UI |
| bits-ui | 2.22.0 | headless primitives |
| paneforge | 1.0.2 | stale ~15 months; wrap it |
| @lucide/svelte | 1.77.0 | ISC license; `@lucide/svelte/icons/<name>` |
| svelte-check | 4.7.8 | peers TS 5/6 — **not 7** |
| typescript | **~6.0.3** | pin; TS 7 breaks svelte-check |

Node ≥20.19 is build-time only; `dist/` is plain static HTML/JS/CSS. Svelte 5 module rule: cannot export a directly reassigned `$state` from `.svelte.ts` — export a non-reassigned object and mutate properties, or export accessors.

---

## 2. Data structures and DTOs

- `api/dto.rs`: `StateSnapshot`, `ProfileSummary`, `ZoneDto`, `AppEntryDto`, `SettingsDto`, `HotkeyDto`, `LogLineDto`, `ApiErrorBody`. Map `Rect`/`KeyEvent`/`Action` to plain DTOs.
- `api/state.rs`: `Inner { draft: AppConfig, last_saved_json: String, applied_profile: String, layout_enabled: bool, selected_zone: i32, history: Vec<Snapshot>, hindex: usize, logs: VecDeque<LogLine> }`.
- `api/worker.rs`: `Command { payload: CommandPayload, reply: oneshot::Sender<Result<Value, ApiError>> }`; `CommandPayload` covers every §5 operation plus `Action` and `MoveWindow` from native.
- `native_loop.rs`: `NativeMessage` enum (`RebuildMenu`, `SetChecked`, `SetToggleLabel`, `ReregisterHotkeys`, `ShowOverlay`, `HideOverlay`, `Shutdown`); `NativeUpdate` is the worker→native variant.
- `session.rs`: token, `HashSet` of session ids, SSE subscriber registry, 60 s reaper.

---

## 3. Frontend component structure

```
frontend/src/
  App.svelte                     root shell + view switching
  lib/
    api/client.ts                fetch wrapper; bootstrap; cookie session; typed DTOs
    api/events.ts                EventSource(withCredentials) + reconnect
    state/app.svelte.ts          exported $state object; $derived dirty; accessors
    components/
      TopBar.svelte  StatusBar.svelte
      LeftRail/{AppLibrary,ProfilesPanel,ToolsDock}.svelte
      Canvas/{ZoneCanvas,ZoneRect,SnapGuides,DragPreview}.svelte
      Inspector/{ZoneInspector,ProcessTab,BrowserTab,MacroTab,Console}.svelte
      dialogs/{WizardDialog,PickerDialog,ConfirmDialog,CloseDialog}.svelte
      views/{HotkeysView,SettingsView}.svelte
```

Layout: left rail, center SVG canvas (max space), right contextual inspector, top status/actions, bottom status. `Resizable.PaneGroup` with collapsible rails; inspector contextual. SVG canvas: `viewBox` from work area, normalized zone coordinates, pointer capture, commit on `pointerup`, snap guides 25/50/75, DnD assignment, context menu, undo/redo via API. No canvas framework.

---

## 4. Implementation tasks

egui dependencies were removed in Phase 6 (the tasks below were executed while they were still present). Each task ends with `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.

| Task | Files | Responsibility | Verification |
|---|---|---|---|
| B1 | `api/state.rs`, `api/commands.rs`, `api/worker.rs`, `api/dto.rs` | Authoritative state, command worker, D1–D6 command logic, D2 dirty contract | Unit tests per command; dirty/clean × discard/cancel matrix; slow command does not block unrelated request |
| B2 | `api/mod.rs`, `api/routes.rs` | Router, all §5 endpoints incl. `/api/shutdown`, ephemeral port, SSE-close watch + graceful shutdown | `oneshot` tests for every endpoint (status + JSON), 401/403/409; test that SSE stream ends on `send_replace(true)` so `serve` returns |
| B3 | `api/middleware.rs` | Token generation, Host validation, session cookie, response headers | Missing token 401, bad token 403, bad Host 403, valid 200 |
| B4 | `api/events.rs` | SSE broadcast, keep-alive, session reaper | Two subscribers receive one event; lagged subscriber skips |
| B5 | `web/assets.rs` | rust-embed static handler, Content-Type, SPA fallback | `/` → index.html; `/assets/x.js` → JS type; missing asset 404 |
| B6 | `session.rs` | Browser launch (`open::that_detached` via `spawn_blocking`), session lifecycle | Session created/reaped; browser close does not stop Rust (manual) |
| B7 | `native_loop.rs` (overlay part) | Overlay layered window on main, `UpdateLayeredWindow` per-zone alpha, `GetAsyncKeyState` gesture detection with variable `MsgWaitForMultipleObjectsEx` timeout (150 ms idle → 16 ms active) | Manual: start Ctrl+drag while idle over another app, overlay appears, tracks, releases; click-through, no taskbar entry; idle CPU negligible (not zero) |
| B8 | `main.rs`, `native_loop.rs` | Main=native thread, message window, queue, safety timer ID 1, manual-reset shutdown event, startup/shutdown per architecture §3.6, reverse updates | Native-loop unit tests (wake-up, modal-dialog message, safety timer, shutdown ordering, event fallback when PostMessageW fails); manual tray/hotkey/shutdown with all threads joined |
| B9 | `system.rs`, `native_loop.rs` | Dynamic tray menu, applied/selected markers, D2 native dialog, hotkey re-registration | Menu rebuild after CRUD; checkmark; D2 matrix; Disable/Enable; Open editor/Settings/Exit |
| F1 | `frontend/` scaffold | Vite/Svelte/Tailwind/shadcn config, scripts | `npm run check` clean; `npm run build` emits `dist/`; no Node needed to serve |
| F2 | `api/client.ts`, `api/events.ts`, `state/app.svelte.ts` | Bootstrap, cookie session, SSE, mirrored state, dirty flag | Store reactivity; dirty flag; SSE reconnect; auth failure paths |
| F3 | `App.svelte`, TopBar, StatusBar, LeftRail, Inspector | Three-pane shell, four status dimensions, contextual inspector | No clipping at 1280×720/1920×1080/maximized; rails collapsible |
| F4 | Canvas components | SVG zone editor, pointer interactions, snap guides, DnD, context menu, undo/redo | All canvas journeys; geometry matches `engine_core`; real drag functional test |
| F5 | dialogs, Inspector tabs | Wizard, picker, D2/D3 confirmations, close dialog, console | Full journey; D1–D6; no data loss |
| F6 | HotkeysView, SettingsView | Editable bindings/settings, autostart, close behavior | Changes persist; hotkeys re-register; autostart syncs |

---

## 5. Dependency decision table

| Package | Version | Purpose | Features/config | Existing? | Decision |
|---|---|---|---|---|---|
| axum | 0.8.9 | HTTP router, SSE, serve | `http1,json,query,tokio` | No | **Add** |
| tokio | 1.54.1 | Async runtime | `rt,macros,net,sync,time`; `current_thread` | No | **Add** (no `rt-multi-thread`/`full`) |
| rust-embed | 8.14.0 | Embed `dist/` | defaults | No | **Add** (no `axum-ex`) |
| open | 5.4.5 | Browser launch | none; no `insecure` | No | **Add** |
| getrandom | 0.3.4 | Token | none on Windows | No | **Add** |
| tokio-stream | 0.1.19 | `BroadcastStream` | `sync` | No | **Add** |
| futures-util | 0.3.34 | StreamExt if used directly | defaults | Transitive via axum | Declare only if used |
| tower | 0.5.3 | `oneshot` in tests | `util` | Transitive | Dev-dep only |
| http-body-util | 0.1.5 | `BodyExt::collect` in tests | none | Transitive | Dev-dep only |
| serde / serde_json | 1.x | API payloads | `derive` | **Yes** | Reuse |
| tracing / tracing-subscriber | 0.1 / 0.3 | Logging | `env-filter` | **Yes** | Reuse |
| global-hotkey | 0.10 | Hotkeys | defaults | **Yes** | Reuse (native thread) |
| tray-icon | 0.26 | Tray | defaults | **Yes** | Reuse (dynamic menu) |
| windows | 0.62 | Win32 | add `Win32_System_LibraryLoader` | **Yes** | Extend features |
| rfd | 0.18 | Native file picker | defaults | **Yes** | Reuse Rust-side |
| eframe / egui / shadcn-egui | 0.37 / 0.37 / 0.8 | Former GUI | — | **Yes** | Removed in Phase 6 |
| tower-http | — | Disk static serving | — | No | **Not needed** (rust-embed) |

Frontend versions and rationale are in architecture §10. No other dependencies are justified.

---

## 6. Test classification and manual verification

### 6.1 Historical baseline (2026-11-22, commit `5e74e85`)

| Command | Result |
|---|---|
| `cargo test` | **43 passed** (40 unit + 3 integration), 0 failed |
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo build --release` | clean |

These tested the **pre-migration egui app and backend**. Superseded by the Phase 6 results below.

### 6.1b Phase 6 verification (2026-11-22, commit `721638f`, egui removed)

| Command | Result |
|---|---|
| `cargo test` | **128 passed** (125 unit + 3 integration), 0 failed |
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo build --release` | clean (3,827,200 B / 3.65 MiB exe) |
| `bun run check` | clean (svelte-check 0 errors/warnings + tsc) |
| `bun test` | **36 passed**, 0 failed |
| `bun run build` | clean (dist emitted and embedded) |

These test the **implemented architecture**: API endpoints and D1–D6 command matrix, native-loop queue/shutdown/D2 tests, config round-trip, web assets and the Svelte frontend.

### 6.2 Planned tests (status at Phase 6)

- **Backend unit/integration — DONE:** every §5 endpoint via `oneshot`; D1–D6 error cases; command state transitions; D2 matrix; native-loop queue/shutdown/D2 tests; shutdown-sequence test (SSE ends → `serve` returns); shutdown-fallback test (`SetEvent` alone latches the wait without an HWND); static handler Content-Type tests. The "message during modal dialog" and "safety timer drains stranded payload" cases remain structural/manual rather than separately automated.
- **Frontend — DONE:** `svelte-check` + `tsc` clean; 36 tests covering store reactivity, dirty flag, SSE reconnect and auth failure paths.
- **Real editing functional test (dev-only) — PENDING:** browser automation (e.g. Playwright, test-only dependency) performing a real SVG zone-boundary drag and asserting backend persistence. This is a functional test, not the `PointerActivity` benchmark.
- **Manual Windows journeys — PARTIAL:** tray-only operation, hotkeys with browser closed, browser open/close lifecycle, overlay Ctrl+drag, full edit→save→apply→launch→tray→disable→enable, unsaved-change prompts and graceful shutdown are implemented; automated coverage is in place for the API/native logic, but the end-to-end GUI journeys are manual checks.
- **Performance — PARTIAL:** Background measured (3 runs, 11.9–12.0 MiB WS / 2.0 MiB private / 3.65 MiB exe); Editor open and Pointer activity **NOT MEASURED** (token not logged, tray automation unreliable, pre-existing browser processes defeat new-PID attribution). See `docs/performance/results.md`. Tokio `current_thread` vs `rt-multi-thread` benchmarking and command-worker channel-overhead measurement were not performed.

No unmeasured value is reported as a result.

---

## 7. Phase gates and acceptance criteria

Gates are defined in architecture §12. Additional task-level acceptance:

- **C1 ✅:** API tests pass; native-loop tests pass; egui UI built and ran during the migration (removed in Phase 6).
- **C2 ✅:** browser loads from embedded assets with no separate server; closing browser leaves Rust running; tray/hotkeys still work.
- **C3 ✅:** canvas journeys pass; geometry matches `engine_core`; real drag functional test is a dev-only manual check (not in the automated suite).
- **C4 ✅:** full user journey; D1–D6 verified by the API command matrix; no data loss (config round-trip tests).
- **C5 ✅:** tray-only works; menu syncs with API changes; D2 dialog works; overlay works; hotkeys fire unfocused; all threads join on exit.
- **C6 ✅:** egui removed; docs match binary; **Background workload measured** (3 runs). Editor open and Pointer activity workloads are **NOT MEASURED** (see `docs/performance/results.md`) — recorded as an honest gap, not a fabricated result.
- **C7 ⬜:** release produced; all verification items checked — Phase 7, pending.

A failing gate blocks the next phase. No workstream begins before its predecessor's gate passes.
