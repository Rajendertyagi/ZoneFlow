# ZoneFlow — Rust + Axum + Svelte Architecture

**Status:** Authoritative architecture and migration plan. **No implementation is authorized by this file.**
**Finalized:** 2026-11-22 · **Plan commit:** `fb5f8d4` (this finalization) · **Source baseline:** `5e74e85` (the pre-finalization state the research was performed against).
**Supersedes:** `slint-gui-migration-plan.md` as the active direction (retained as historical research). Product behavior remains governed by the frozen documents: `zoneflow-product-workflow.md` (D1–D6), `zoneflow-feature-preservation.md`, `zoneflow-implementation-roadmap.md`, and `zoneflow-gui-architecture.md` (egui-specific guidance, historical for the web UI).
**Companion:** `zoneflow-rust-axum-svelte-implementation-plan.md` holds code-level API details, task breakdown and test classification. This document is the single authoritative definition of architecture decisions; the companion must not restate them differently.

---

## 1. Goal and frozen constraints

Replace the resident egui/eframe GUI with a **headless Rust background process** that serves an on-demand Svelte editor over a local HTTP API. The browser opens only when the user requests the editor; closing it must not terminate Rust. Tray, global hotkeys, Win32 window management, autostart and config compatibility are preserved.

**Verified motivation:** the headless Rust + Axum spike measured ~2.0–2.1 MiB private / ~10.9–12.5 MiB working set versus ~59–62 MiB private / ~109–112 MiB working set for the current egui app. That is the always-on process only — not the complete application with a browser open (§9).

**Frozen constraints:**
- No Node.js/Bun in the installed app — build-time only. No Electron, Tauri, WebView or embedded browser.
- No separate frontend server in production; Rust serves embedded `dist/`. No database (none is needed).
- Primary monitor only. No multi-monitor.
- D1–D6 and all feature-preservation requirements remain in force (§6).
- Priorities: functional correctness → reliability → maintainability → performance → proportionate security.

---

## 2. Current architecture assessment (verified from source)

The crate is a binary (`src/main.rs`, no `src/lib.rs`). The backend is already toolkit-independent:

| Module | GUI dependency | Migration impact |
|---|---|---|
| `src/engine_core.rs` | None (only `config::{Profile, Zone}`) | Unchanged |
| `src/engine.rs` | None (`windows`, `config`, `engine_core`) | Unchanged |
| `src/config.rs` | None (`serde`, `fs`, `OnceLock`/`RwLock`) | Unchanged |
| `src/system.rs`, `src/autostart.rs` | None (`global-hotkey`, `tray-icon`, Win32) | Refactored for dynamic menu + native thread (§4) |
| `src/main.rs`, `src/editor/*`, `src/overlay.rs` | `eframe`, `egui`, `shadcn-egui`, `rfd` | Replaced by HTTP API + Svelte UI + native overlay |

**Serde status:** `AppConfig`, `Settings`, `Profile`, `Zone`, `ResolutionPreset` derive `Serialize`/`Deserialize`; `engine_core::Rect`, `KeyEvent` and `system::Action` do **not**. The HTTP layer needs its own DTOs (§5.1) — do not assume every backend type can go straight into `Json`.

**Verified source facts that constrain the design:**
- `src/system.rs` builds a **fixed** menu and matches menu IDs by display label — both must change (§4.4).
- `tray-icon` 0.26 `TrayIcon` and all muda menu objects are `!Send`/`!Sync`; `GlobalHotKeyManager` is `Send + Sync` but must be created on the message-loop thread.
- `set_event_handler` in both crates is a one-way `OnceCell` switch — **never call it**; use `receiver()` with a single consumer.
- `register_all` is fail-fast; keep the existing per-item loop with `failures` tracking.
- No Win32 message loop exists in the project today — the native thread is required for **correctness**, not just organization.
- Remaining backend bugs (B4 gesture guards, B6 config-source unification, B10 `restore_active` parity, B12 dead `is_auto_start()`) stay tracked in the roadmap; the new UI must not reintroduce them.

---

## 3. Target architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Rust executable — 3 threads                                             │
│                                                                          │
│  Thread 1: MAIN = NATIVE (owns ALL Win32 windows)                        │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │ System: tray icon + global hotkeys                                 │  │
│  │ Hidden message-only window (wake-up + safety timer)                │  │
│  │ Overlay layered window (Ctrl+drag zone highlight)                  │  │
│  │ MsgWaitForMultipleObjectsEx → PeekMessage → Translate/Dispatch     │  │
│  │ WM_APP: drain queue · MsgWait timeout: drag poll · WM_STOP: quit  │  │
│  └───────┬──────────────────────────────────────────▲─────────────────┘  │
│          │ Actions / MoveWindow (mpsc)             │ NativeUpdate (mpsc) │
│          │                                         │ + PostMessageW wake │
│  ┌───────▼─────────────────────────────────────────┴──────────────────┐  │
│  │ Thread 2: COMMAND WORKER                                           │  │
│  │ Authoritative Inner state · history · synchronous engine calls     │  │
│  │ Receives Command + oneshot reply; broadcasts SSE events            │  │
│  └───────┬──────────────────────────────────────────▲─────────────────┘  │
│          │ oneshot replies                         │ Command + oneshot   │
│  ┌───────▼─────────────────────────────────────────┴──────────────────┐  │
│  │ Thread 3: TOKIO (current_thread runtime)                           │  │
│  │ Axum HTTP API · /api/* · /api/events (SSE) · embedded dist/        │  │
│  └───────┬────────────────────────────────────────────────────────────┘  │
└──────────┼───────────────────────────────────────────────────────────────┘
           │ loopback HTTP, per-launch token
    ┌──────▼───────┐
    │ Browser      │  opened on demand; closing it does not stop Rust
    │ Svelte 5 SPA │  SVG canvas · shadcn-svelte · Tailwind 4
    └──────────────┘
```

### 3.1 Thread ownership (authoritative — one definition)

There are exactly **three threads**. The main thread **is** the native thread; there is no separate overlay thread and no separate "native thread" beyond main.

| Thread | Creates and owns | Runs | Never does |
|---|---|---|---|
| **1 — main / native** | `System` (tray + hotkeys), hidden message-only window, overlay layered window, local cached zones/work area/alphas | `MsgWaitForMultipleObjectsEx` loop; `WM_APP` queue drain; timeout drag poll (§3.7); `System::poll()`; `WM_STOP` shutdown | Call engine functions that block; read authoritative state directly; call `PostQuitMessage` from another thread |
| **2 — command worker** | Authoritative `Inner` state, history, engine calls | Serialized synchronous engine operations (including the 8 s launch wait) | Touch Win32 windows or tray/hotkey objects; block a Tokio worker |
| **3 — Tokio** | Axum server, SSE subscribers, HTTP handlers | `axum::serve` on a `current_thread` runtime | Run blocking engine work or blocking browser launch on the runtime thread |

**Why main = native:** `tray-icon` and `global-hotkey` require a message loop on the thread that creates their managers; the overlay and hidden window also require their creating thread to pump messages. One thread can own all of them and dispatch all their messages in one loop. This removes a whole thread, removes cross-thread `HWND` marshalling for the overlay, and is the simplest reliable arrangement. The previous documents' "separate overlay thread" and "main thread only coordinates" descriptions are **superseded** by this section.

### 3.2 Startup ordering and readiness

1. `main` loads config, generates the 256-bit token, creates channels: `cmd_tx/cmd_rx` (worker), `native_tx/native_rx` (payload queue sender + receiver), `shutdown_tx/rx` (oneshot for Axum).
2. `main` spawns **thread 2** (command worker), initializing `Inner` from config.
3. `main` spawns **thread 3** (Tokio `current_thread` runtime): bind `127.0.0.1:0`, read `local_addr().port()`, build the router, then `axum::serve(...).with_graceful_shutdown(...)`. It sends `Bound { port, token }` back to main over a oneshot **before** awaiting `serve`.
4. `main` blocks on that oneshot (bounded timeout), then creates the tray, hotkeys, hidden window and overlay window **on main**, starts the safety timer (ID 1), publishes the `HwndSend` handle to producers, and enters the `MsgWaitForMultipleObjectsEx` message loop (§3.7).
5. No producer may post to the hidden window before step 4 publishes the handle — the oneshot gate enforces this.

### 3.3 Communication paths (channels only — no event framework)

1. **Native → worker:** `System::poll()` yields `Action`s (hotkey, tray clicks); the drag-poll timeout (§3.7) yields `MoveWindow { hwnd, zone }` on release. Both go over `cmd_tx` (mpsc).
2. **HTTP → worker:** each handler sends `Command { payload, reply: oneshot::Sender<Result<...>> }` and awaits the reply. Blocking engine work never runs on the Tokio thread.
3. **Worker → HTTP:** state changes are broadcast on `tokio::sync::broadcast::Sender<String>` (Send + Sync, held by the worker) and streamed via `GET /api/events`. Results return via the oneshot reply.
4. **Worker → native (reverse path):** profile CRUD/selection, hotkey edits, settings changes and Disable/Enable produce `NativeUpdate` payloads. The worker pushes each payload into the native payload queue and posts `WM_APP` to the hidden window. The native thread drains the queue and rebuilds the tray submenu, updates the applied checkmark, toggles the Disable/Enable label and re-registers hotkeys. The native thread never reads authoritative state directly.
5. **Shutdown:** governed by the single ordered sequence in §3.6. Both tray "Exit" and an API shutdown request converge on it; no other shutdown ordering is valid.

### 3.4 Native message mechanism (reliable wake-up)

An `mpsc` send does not wake a thread blocked in `GetMessageW`, and `PostThreadMessageW` messages are **lost** when the thread enters a modal loop such as `MessageBoxW` (Microsoft Learn, `PostThreadMessageW` remarks). Therefore:

- **Hidden message-only window:** `RegisterClassW` (unique class name; tolerate `ERROR_CLASS_ALREADY_EXISTS` = 1410), `CreateWindowExW(..., Some(HWND_MESSAGE), ...)`. Message-only windows receive posted messages and are not visible/enumerable.
- **Payload queue:** `Arc<Mutex<VecDeque<NativeMessage>>>`, bounded (e.g. 256). `WM_APP` is a **wake-up signal only** (no payload in `LPARAM`, avoiding pointer-ownership hazards). **Overflow behavior:** `NativeMessage` variants are either *replaceable state notifications* (`RebuildMenu`, `SetChecked`, `SetToggleLabel`, `ReregisterHotkeys`, `ShowOverlay`/`HideOverlay` — all idempotent full-state refreshes) or *must-not-drop events* (currently none in the queue; `Shutdown` is the `WM_STOP` window message, and command results use `oneshot`). On overflow, coalesce: remove older replaceable notifications of the same kind and keep the newest full-state value for each; never drop a must-not-drop event. Because every queued item is an idempotent refresh, applying only the newest of each kind leaves tray state, hotkey registrations and overlay state correct — no stale state from dropping an arbitrary oldest item.
- **Posting:** producers check an `Arc<AtomicBool>` accepting flag, push the payload, then `PostMessageW(Some(hwnd), WM_APP, 0, 0)` and check the `Result`. On failure, log and rely on the safety timer — never spin.
- **Safety timer (ID 1, always on):** `SetTimer(hidden_hwnd, TIMER_SAFETY_DRAIN=1, 100, None)` → `WM_TIMER` drains the queue. `WM_TIMER` is low-priority and coalesced, which is desirable for a safety net; it survives modal loops. This guarantees a payload is never silently stranded even if a `PostMessageW` wake-up is lost or rejected (queue full, 10,000-message limit).
- **Drag polling is NOT a second `WM_TIMER`:** `WM_TIMER` is the lowest-priority message and is only generated when no higher-priority messages are pending, so a "30 ms" timer is neither 30 ms nor reliable during a drag. Instead the message loop uses `MsgWaitForMultipleObjectsEx` with a short timeout **only while a drag is armed/active** (§3.7). Idle uses `INFINITE` → zero CPU.
- **WndProc:** `WM_APP` and `WM_TIMER(ID 1)` only drain the queue and dispatch to non-blocking handlers. No engine calls, no blocking I/O, no locks held across dispatch.
- **D2 modal dialog:** `MessageBoxW` is modal but its loop dispatches window messages, so `WM_APP`/`WM_TIMER` still arrive. Use `None` (not `HWND_MESSAGE`) as the message box owner — `IsWindow(HWND_MESSAGE)` is false.
- **HWND sharing:** `HwndSend` newtype wrapping the raw pointer (`HWND` is `!Send`/`!Sync`), with `unsafe impl Send/Sync` justified by the accepting flag and join ordering. `PostMessageW` from another thread is safe (OS marshals by HWND).
- **Loop:** `MsgWaitForMultipleObjectsEx` with `QS_ALLINPUT` and a drag-dependent timeout (§3.7); drain queued messages with `PeekMessageW(..., PM_REMOVE)` → `TranslateMessage` → `DispatchMessageW`. Never filter `PeekMessageW` by the hidden HWND (that would hide `WM_QUIT`, whose `hwnd` is NULL) or by a range excluding `WM_APP`. Break when `WM_QUIT` is retrieved. (A plain `GetMessageW(&mut msg, None, 0, 0)` loop is the simpler alternative if drag polling is moved off-thread; §3.7 selects the `MsgWait` form for responsive, low-CPU drag.)
- **Teardown:** governed by §3.6 — the hidden window and its timers stay alive until every producer has been joined; `DestroyWindow` is the last native step, after which `PostQuitMessage(0)` is called on main.

### 3.5 Blocking-work isolation

The engine can block up to 8 s waiting for a launched window. All such calls run on **thread 2** (command worker), serialized. Axum handlers only send commands and await oneshot replies. Browser launch (`open::that_detached`) and disk I/O run via `tokio::task::spawn_blocking` so the `current_thread` runtime is never stalled. `spawn_blocking` uses Tokio's auxiliary blocking-pool threads — so the process has **three primary application threads plus short-lived blocking-pool threads** for one-off blocking calls. The long-lived command worker is a `std::thread`, not a blocking-pool task (Tokio docs recommend dedicated threads for persistent loops).

### 3.6 Shutdown state machine (authoritative — one ordered sequence)

This is the **only** shutdown definition. All other sections reference it. It is driven by an idempotent `AtomicBool SHUTTING_DOWN` guard so tray Exit and API shutdown converge. Verified against Axum 0.8.9 `serve` source: `with_graceful_shutdown` stops accepting connections, then waits for **all** connection tasks via `close_tx.closed().await`; an endless SSE stream would hang that await forever, and there is **no built-in timeout**. Therefore SSE streams must be explicitly closed **before** firing graceful shutdown.

**Triggers:** tray "Exit" (main thread) or `POST /api/shutdown` (Tokio thread). Both execute steps 1–2 then post `WM_STOP` to the hidden window; the rest runs on main after the message loop exits.

1. **Guard + stop new work:** set `SHUTTING_DOWN = true`; set the native queue `accepting = false` (under its mutex, so an in-flight producer either completes its push before the flag flips or skips — no post-after-destroy race).
2. **Close SSE streams:** `sse_tx.send_replace(true)` on a `tokio::sync::watch` channel each SSE handler selects on via `StreamExt::take_until`. This ends every SSE response so its connection task can finish. (Use `send_replace`, not `send`, which errors with no receivers.)
3. **Signal Axum:** fire the `axum_shutdown` oneshot → axum stops accepting new connections and drops the listener itself, then waits for active connection tasks.
4. **Wake native loop:** `PostMessageW(hidden_hwnd, WM_STOP, 0, 0)` (check result). The WndProc calls `PostQuitMessage(0)` **on main** — `WM_QUIT` is posted by the owning thread and is never dispatched as a normal window message. `WM_STOP` does **not** destroy any window.
5. **Exit loop:** the `MsgWaitForMultipleObjectsEx` loop (§3.7) retrieves `WM_QUIT` via `PeekMessageW` and breaks; main leaves the loop. The hidden window and timers are still alive.
6. **Join Tokio:** `server_thread.join()` — `axum::serve` returns only after all connection tasks (SSE now closed, in-flight requests drained) finish. No handler waits on main, so no deadlock.
7. **Stop worker:** drop main's `cmd_tx` clone (all handler clones are already dropped with their finished requests); the worker drains queued commands and exits. `worker.join()` — the worker's native-update sender and SSE broadcast sender drop here, so **all producers to the native queue are now stopped**.
8. **Final native drain:** drain any remaining payloads under the queue lock and apply them (idempotent). No producer can add more.
9. **Kill timer:** `KillTimer(hidden, TIMER_SAFETY_DRAIN)`. Drag polling is a loop timeout, not a timer, so it stops when the loop exits at step 5. `DestroyWindow` also destroys timers, but explicit kill stops generation immediately; WndProc tolerates any already-queued `WM_TIMER`.
10. **Destroy windows:** `DestroyWindow(overlay)` then `DestroyWindow(hidden)` — safe because every producer is joined and `accepting = false`.
11. **Drop native objects:** let `System` (TrayIcon, Menu, GlobalHotKeyManager) drop on main — their `Drop` calls `DestroyWindow` on their own HWNDs, on the creating thread. `UnregisterClassW` is optional at process exit.
12. **Exit:** main returns; process ends.

**No circular wait:** Tokio waits only on connection tasks (which end at step 2); the worker waits only on its command channel (closed at step 7); main waits on Tokio then worker (steps 6–7) before touching any HWND (step 10). Each component terminates independently of the step that follows it.

### 3.7 Drag polling (responsive, low idle CPU)

Two distinct responsibilities, two mechanisms:

- **Safety drain:** always-on `WM_TIMER` ID 1 (§3.4). Low priority is fine.
- **Drag detection / overlay updates:** the message loop calls `MsgWaitForMultipleObjectsEx(None, timeout, QS_ALLINPUT, default())`. When a drag is armed/active, `timeout = 16 ms` and a `WAIT_TIMEOUT` result calls `poll_drag_state()` (`GetAsyncKeyState(VK_CONTROL) < 0 && GetAsyncKeyState(VK_LBUTTON) < 0`, then `GetCursorPos`, then update the overlay). When idle, `timeout = INFINITE` → the thread blocks with zero CPU. Input messages wake the loop via `QS_ALLINPUT`; use `MWMO_INPUTAVAILABLE` if a drain-then-wait pattern is used. `GetAsyncKeyState` reflects state at call time (unlike `GetKeyState`); ignore its unreliable low bit. No low-level mouse hook (global, timeout-fragile) and no always-on drag timer.

---

## 4. Tray and profile workflow (D1–D6)

`src/system.rs` currently builds a fixed menu and matches IDs by label. The new design, executed on main from `NativeUpdate`:

```
ZoneFlow Studio — Applied: <applied> · Selected: <selected>
────────────────────────────
Profiles              ▸  Website ✓   (checkmark = applied)
                          Coding •    (bullet = selected, when different)
                          Trading
────────────────────────────
Disable layout           (toggles to Enable layout)
Open editor…
Settings…
Exit
```

- Rebuild the profile submenu after every profile create/rename/delete/select. Use stable IDs (`MenuId::new("profile:<name>")`), never label matching. Show Applied and Selected separately — critical when disabled (selection changes Selected only).
- **D6:** tray click applies when enabled, selects only when disabled. Home `select` and rail `load` remain distinct API operations (§5.2).
- **D2:** a dirty-draft switch returns `409 dirty_state` with no state change; the native thread shows a native `MessageBoxW` Discard/Cancel dialog and retries with `discard_dirty=true` on Discard (§5.2).
- **D4:** Disable/Enable suspends all automation; runtime-only; label toggles.
- Open editor / Settings open the browser editor; Exit triggers §3.3 shutdown.

---

## 5. API contracts

All responses are JSON. All `/api/*` requests require the session (§8). Errors are `{ "error": { "code": string, "message": string } }`.

### 5.1 DTOs and transport

- Backend types without Serde (`Rect`, `KeyEvent`, `Action`) are mapped to plain DTOs in `api/dto.rs`; never serialized directly.
- `GET /api/state` → draft config, dirty, applied, enabled, selected, profiles, zones, settings, hotkeys, logs, engine status.
- `GET /api/events` → SSE `{type, payload}` events: `state-changed`, `log`, `status`, `hotkey-action`, `apply-result`. Each handler selects on a `watch` shutdown receiver via `take_until` so shutdown can end the stream (§3.6 step 2).
- `GET /api/status` → lightweight fallback poll.
- `POST /api/shutdown` → triggers the §3.6 sequence (returns 200 before the server stops).
- **Transport:** SSE for backend→UI events, `fetch()` for commands. Native `EventSource` cannot set a bearer header, so it uses the session cookie (§8). WebSocket and polling are unnecessary. One `EventSource` per client (HTTP/1 limits ~6 connections per origin).

### 5.2 Profiles (D6)

| Method | Path | Behavior |
|---|---|---|
| GET | `/api/profiles` | `[{name, zone_count, applied, selected}]` |
| POST | `/api/profiles` | create `{name}` |
| PUT | `/api/profiles/{name}` | rename `{new_name}` |
| DELETE | `/api/profiles/{name}` | delete; clear Applied if it was applied |
| POST | `/api/profiles/{name}/duplicate` | copy `{new_name}` |
| POST | `/api/profiles/{name}/load?discard_dirty=false` | **rail:** load into draft; dirty-guarded |
| POST | `/api/profiles/{name}/select?discard_dirty=false` | **Home:** selection only; dirty-guarded |
| POST | `/api/profiles/{name}/apply?discard_dirty=false` | **tray:** apply when enabled, select-only when disabled; dirty-guarded |

**D2 unified contract:** worker owns dirty state. Clean → execute. Dirty + `discard_dirty=false` → `409 dirty_state`, no state change. Dirty + `discard_dirty=true` → revert draft, then execute. Web SPA shows a shadcn Dialog on 409; tray shows native `MessageBoxW`. Same flow for all three operations.

### 5.3 Zones

`GET /api/zones`; `POST /api/zones`; `PUT /api/zones/{id}`; `DELETE /api/zones/{id}` (refuse last); `POST /api/zones/{id}/split {axis,ratio}`; `POST /api/zones/merge {source_id,target_id}`; `POST /api/zones/{id}/assign {label,exe,path}` (D3 rules); `DELETE /api/zones/{id}/assignment`; `POST /api/undo`, `/api/redo`.

### 5.4 Applications

`GET /api/apps?source=installed|running` (cached, 2 s TTL for running); `POST /api/apps/browse` (Rust-side `rfd` picker). Discovery moves out of `rail.rs` into a pure Rust module; the library is runtime enumeration, never persisted assignments.

### 5.5 Actions (D1–D5)

`POST /api/save` (persist only); `/api/apply` (D1 arrange running only, 409 if dirty, inactive when disabled); `/api/launch-arrange` (D1 explicit launch + arrange, 409 if dirty); `/api/preview` (D5 inspect without running); `/api/revert`; `/api/disable`, `/api/enable` (D4 runtime-only).

### 5.6 Settings, hotkeys, logs

`GET/PUT /api/settings` (PUT syncs autostart); `GET/PUT /api/hotkeys` (PUT re-registers via native thread); `GET /api/logs?since=N`.

---

## 6. Frozen product decisions (unchanged)

- **D1** Apply (arrange running only) and Launch & Arrange (explicit launch) are distinct.
- **D2** Apply disabled while dirty; profile switching requires explicit Discard/Cancel; never silently discard.
- **D3** Replace assignment preserves geometry/ID/position, resets rules with confirmation, never deletes the library entry.
- **D4** Disable suspends all automation; runtime-only; selection while disabled changes Selected only.
- **D5** Launch performs real execution; separate Preview inspects steps; no simulated success.
- **D6** rail = load draft; Home = select only; tray = apply when enabled, select-only when disabled.
- Same-executable multi-window limitation remains documented; no multi-window claim. Primary monitor only.

---

## 7. Frontend and SVG canvas

The frontend component tree, Svelte 5 patterns and layout are defined **once** in the companion plan §3 — this document does not duplicate them. Architecture-level canvas rules: zones are stored as normalized percentages; the SVG `viewBox` comes from `engine::work_area()`; geometry rules stay in `engine_core`. Temporary pointer previews stay in frontend `$state` (no HTTP per move); commit on `pointerup`. No large canvas framework — SVG suffices for 3–10 rectangles.

---

## 8. Security (proportionate to a local utility)

Loopback-only bind `127.0.0.1:0` (never `0.0.0.0`/`::`); use literal `http://127.0.0.1:<port>`. Per-launch 256-bit token in the URL **fragment** → `POST /api/session` with bearer → `HttpOnly; SameSite=Strict` cookie → `history.replaceState()` clears the fragment → fetch and `EventSource` use the cookie. Guard covers **only `/api/*`**; static assets are public. Validate `Host` exactly (DNS-rebinding mitigation); no CORS headers, never `*`; `Referrer-Policy: no-referrer`, `X-Content-Type-Options: nosniff`. Open the browser with `open::that_detached` (no `insecure` feature). Reap stale SSE sessions after 60 s. Failure behavior (missing/invalid token, failed exchange, expired session, reconnect) is in the companion plan §1.1. Localhost alone is not complete security; no further hardening is warranted for this utility.

---

## 9. Performance strategy

Three workloads, measured and reported separately (`docs/performance/results.md`, `benchmark.ps1`):

1. **Background** — Rust backend, tray, hotkeys, HTTP server; browser closed. Measured: ~2.0–2.1 MiB private / ~10.9–12.5 MiB WS for the spike vs ~59–62 MiB private / ~109–112 MiB WS for egui.
2. **Editor open** — Rust + browser with full Svelte UI. **Not yet measured.**
3. **Pointer activity** — editor open + 20 s synthetic cursor movement. **Not yet measured.** This is **not** editing; real SVG drag verification is a separate dev-only functional test.

Metrics: working set, private bytes, CPU (Rust and browser separately, including browser CPU for Editor open), cold startup, exe size. ≥3 runs, report ranges. **Never present the headless backend's RAM as the complete application's memory.**

---

## 10. Dependency decisions

Full table with versions, features, links, existing/new status and rationale is in the companion plan §5. Summary: **add** `axum 0.8.9`, `tokio 1.54.1` (`rt, macros, net, sync, time` — `current_thread`, no `rt-multi-thread`/`full`), `rust-embed 8.14.0`, `open 5.4.5`, `getrandom 0.3.4`, `tokio-stream 0.1.19` (`sync`); **reuse** `serde`, `serde_json`, `tracing`, `tracing-subscriber`, `global-hotkey 0.10`, `tray-icon 0.26`, `windows 0.62` (add `Win32_System_LibraryLoader` for `GetModuleHandleW`); **keep until Phase 6** `eframe`, `egui`, `shadcn-egui`, `rfd`. Frontend: Svelte 5.59.1, Vite 8.3.8, plugin 7.3.1, Tailwind 4.3.5, shadcn-svelte 1.8.1, Bits UI 2.22.0, Paneforge 1.0.2 (wrapped), `@lucide/svelte` 1.77.0, **TypeScript pinned `~6.0.3`** (TS 7 is outside `svelte-check`'s peer range).

---

## 11. Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Browser-on-demand UX unusual | High | Accept explicitly; `--app=` where available; tray is primary interface |
| Native overlay correctness | High | Main/native thread owns it; manual Windows tests (companion §6) |
| D1–D6 parity lost in rewrite | High | API contracts encode rules; per-decision checkpoint tests |
| Paneforge stale (~15 months) | Medium | Isolate behind `Resizable` wrapper; replaceable |
| Native file picker from web UI | Medium | Keep `rfd` Rust-side via `/api/apps/browse` |
| Two runtimes to debug | Medium | Clear API boundary; SSE event log; API integration tests |
| Remaining backend bugs B4/B6/B10/B12 | Medium | Track in roadmap; do not reintroduce |

---

## 12. Migration phases and gates

egui dependencies are **not removed until Phase 6**; the app stays buildable throughout.

| Phase | Scope | Gate |
|---|---|---|
| 0 — Foundation ✅ DONE | Headless spike measured; benchmark harness + results | Background RAM reduction verified; architecture approved |
| 1 — Backend API | Add Axum/Tokio/rust-embed; `api/`, `web/`, `session`; all §5 endpoints; native thread + command worker + message window; egui still runs | fmt/clippy/test/release clean; API + native-loop tests pass; egui UI still works |
| 2 — Frontend scaffold | Svelte/Vite/Tailwind/shadcn project; build `dist/`; embed; serve; browser on demand | Browser loads from embedded assets with no separate server; closing browser leaves Rust running |
| 3 — SVG canvas | Zones, move/resize/split/merge, snap guides, draw, DnD, context menu, undo/redo | All canvas journeys pass; geometry matches `engine_core`; real drag functional test passes |
| 4 — Panels/dialogs | Left rail, inspector, wizard, picker, confirmations, close dialog, Hotkeys/Settings | Full journey; D1–D6 verified; no data loss |
| 5 — Lifecycle/native | Dynamic tray, D2 dialog, reverse updates, overlay, session, graceful shutdown | Tray-only works; menu syncs; dirty switch confirms; overlay works; all threads join |
| 6 — Verification/removal | Measure three workloads; remove egui deps; sync docs | All checks clean; performance recorded; docs match binary |
| 7 — Release hardening | Installer/signing, CI artifacts | Release produced; all verification items checked |

A failing gate blocks the next phase.

---

## 13. Verification checklist

- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` clean at every gate.
- [ ] API integration tests cover every §5 endpoint and D1–D6 error cases (409 dirty, disabled guards).
- [ ] Native-loop tests: wake-up drains queue; message posted during D2 dialog is processed after it closes; safety timer drains a stranded payload; shutdown sequence joins all threads and destroys windows only after producers stop.
- [ ] Shutdown test: SSE stream ends on `sse_tx.send_replace(true)` so `axum::serve` graceful shutdown returns (no hang); both tray Exit and `/api/shutdown` converge on §3.6.
- [ ] Browser loads from embedded assets with no Node/Bun and no separate server; closing it does not stop Rust.
- [ ] All canvas interactions work; geometry matches `engine_core`; real drag functional test passes.
- [ ] Three performance workloads measured and reported separately.
- [ ] No source/dependency changes made by this planning task.

---

## 14. Official research sources

- Microsoft Learn: message-only windows, `CreateWindowExW`, `RegisterClassW`, `PostMessageW`, `PostThreadMessageW` (modal-loop loss remark), `GetMessageW`, `PostQuitMessage`, `MessageBoxW`, `WM_QUIT`, "About Messages and Message Queues"; Raymond Chen (Old New Thing) on window vs thread messages and `HWND_MESSAGE` ownership.
- windows-rs 0.62 docs/source: window/message APIs, `WNDCLASSW`, `WNDPROC`, `HWND` `!Send`/`!Sync`, required Cargo features.
- `tray-icon` 0.26 / `global-hotkey` 0.10 / muda 0.21 docs + published source: thread requirements, `set_menu`, `CheckMenuItem::set_checked`, `with_id`, `set_event_handler` OnceCell behavior, `register_all` fail-fast, `Send`/`Sync`, `Drop` behavior.
- crates.io/docs.rs: axum 0.8.9, tokio 1.54.1, rust-embed 8.14.0, open 5.4.5, getrandom 0.3.4, tokio-stream 0.1.19, tower 0.5.3, http-body-util 0.1.5.
- npm registry + official docs: Svelte 5.59.1, Vite 8.3.8, plugin 7.3.1, Tailwind 4.3.5, shadcn-svelte 1.8.1, Bits UI 2.22.0, Paneforge 1.0.2, `@lucide/svelte` 1.77.0, svelte-check 4.7.8, TypeScript 6.0.3 pin.
- MDN: Same-Origin Policy, CORS, SSE, `EventSource` credentials and connection limits.
