# ZoneFlow — Rust + Axum + Svelte: Detailed Implementation Plan

**Status:** Implementation-ready companion to the architecture plan. **No implementation is authorized by this file** — it exists so that, once approved, work can proceed without another research cycle.
**Date:** 2026-11-21 · **Corrected:** 2026-11-22 after two independent audits (Windows message pump, browser auth bootstrap, blocking-work isolation, D6 profile actions, overlay constant, dependency/error/DTO/benchmark corrections; then fragment-only auth, native-thread ownership, dynamic tray + D2 dialog, and honest pointer-activity benchmark).
**Relationship:** Implements `zoneflow-rust-axum-svelte-architecture.md`. Product behavior from `zoneflow-product-workflow.md` (D1–D6) and `zoneflow-feature-preservation.md` remains authoritative.
**Research basis:** official docs.rs, crates.io, svelte.dev, vite.dev, tailwindcss.com, shadcn-svelte.com, bits-ui.com, paneforge.com, lucide.dev, Microsoft Learn — all fetched 2026-11-21. Code below is either verbatim from those sources or a composition of verified primitives; compositions are marked where relevant.

---

## 1. Verified API reference

### 1.1 Axum 0.8.9

**Cargo baseline** (verified feature flags):

```toml
[dependencies]
axum = "0.8.9"                       # defaults include json, query, tokio, http1, matched-path, tracing
tokio = { version = "1.54.1", features = ["rt-multi-thread", "macros", "net", "sync", "time"] }
tokio-stream = { version = "0.1", features = ["sync"] }   # BroadcastStream
futures-util = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
getrandom = "0.3"                    # per-launch token (32 bytes from OS CSPRNG)
rust-embed = "8.14.0"                # default features; do NOT enable axum-ex
open = "5"                           # browser launch; keep insecure feature OFF

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }          # ServiceExt::oneshot
http-body-util = "0.1"                                    # BodyExt::collect
```

**Router + State.** `Router<S>` means "missing state S"; only `Router<()>` can be passed to `serve`. Call `.with_state(...)` at the top level, not inside router-returning functions. `State` must precede body extractors. Share state with `Arc<RwLock<Inner>>`; **drop the guard before any `.await`** (a `std::sync` guard held across await makes the future `!Send`).

```rust
use axum::{extract::State, routing::{get, post}, Router};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
struct AppState { inner: Arc<RwLock<Inner>> }

#[derive(Default)]
struct Inner { /* draft config, dirty, applied, enabled, selection, logs */ }

async fn get_state(State(s): State<AppState>) -> String {
    let snapshot = { s.inner.read().unwrap().clone() }; // guard dropped here
    serde_json::to_string(&snapshot).unwrap()
}

let app = Router::new()
    .route("/api/state", get(get_state))
    .with_state(state); // -> Router<()>
```

**JSON, Path, Query.** `Json<T>` must be the **last** extractor (consumes body); sets `Content-Type: application/json`. Route syntax in 0.8 is `{id}` / `{*rest}` (not `:id`). Only one `Path<T>` per handler. `Query<T>` parse failure → 400.

**Custom error with 409 for dirty state** (composition of the verified `IntoResponse` pattern):

```rust
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

#[derive(Debug)]
enum ApiError { DirtyState, Unauthorized, Forbidden, NotFound(String), Internal(String) }

impl ApiError {
    fn parts(&self) -> (StatusCode, &'static str, String) {
        match self {
            ApiError::DirtyState => (StatusCode::CONFLICT, "dirty_state", "save or discard changes first".into()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", "missing token".into()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "invalid token or host".into()),
            ApiError::NotFound(w) => (StatusCode::NOT_FOUND, "not_found", format!("{w} not found")),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", m.clone()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = self.parts();
        // Matches the architecture contract: { "error": { "code": ..., "message": ... } }
        (status, Json(json!({ "error": { "code": code, "message": message } }))).into_response()
    }
}
// Result<Json<T>, ApiError> works directly as a handler return type.
```

**Auth + Host middleware — bootstrap/session design.** The guard applies **only to `/api/*` routes**, never to static assets. The browser cannot attach a bearer header to its initial navigation or to a native `EventSource`, so authentication uses a two-step bootstrap:

1. Rust generates a 256-bit token and opens `http://127.0.0.1:<port>/#token=<token>`. The token is in the **URL fragment**, which browsers never send in HTTP requests, logs or `Referer`.
2. The SPA reads `location.hash` in JS, then calls `POST /api/session` with `Authorization: Bearer <token>`. The server validates it and sets an `HttpOnly; SameSite=Strict` session cookie containing the same token.
3. On a successful exchange the SPA calls `history.replaceState(null, '', location.pathname)` to **clear the fragment** so the token is no longer visible in the address bar or retained in the URL.
4. All later requests — including `fetch()` and native `EventSource` — are same-origin and carry the cookie automatically (`EventSource` with `withCredentials: true`). The middleware accepts **either** the bearer header **or** the session cookie.
5. Static assets (`/`, `/assets/*`) are public — they contain no secrets. The token dies with the process; the cookie dies with the browser session.

**Failure behavior (must be implemented and tested):**

| Condition | Behavior |
|---|---|
| No fragment / missing token | Show a clear error page: "Open the editor from the ZoneFlow tray menu." Do not call `/api/session`. |
| Invalid token (bad exchange) | `/api/session` returns 403; show "Session rejected — reopen the editor from the tray." Do not retry in a loop. |
| Failed exchange (network/server error) | Show a retry button that re-reads `location.hash` and retries once; if still failing, instruct the user to reopen from the tray. |
| Expired / invalid session on later request | API returns 401; the SPA shows a session-expired banner with a "Reopen from tray" action. Do not silently clear the draft. |
| SSE disconnect | `EventSource` reconnects automatically; the cookie authenticates each reconnect. After repeated failures, show a "connection lost" indicator and keep local state intact. |

The token is per-launch and lives only as long as the Rust process; there is no separate expiry timer. "Expired session" means the process restarted (old cookie no longer matches the new token).

Use `middleware::from_fn_with_state` with `route_layer` on an `/api` sub-router (added **after** routes, so auth returns 401/403 rather than converting 404). Last extractor must be `Request`.

```rust
use axum::{extract::{Request, State}, http::{header, StatusCode},
           middleware::{self, Next}, response::{IntoResponse, Response}};

async fn guard(State(s): State<AppState>, req: Request, next: Next) -> Response {
    let host_ok = req.headers().get(header::HOST).and_then(|v| v.to_str().ok())
        .map(|h| h == s.allowed_host.as_str()).unwrap_or(false);
    if !host_ok { return (StatusCode::FORBIDDEN, "bad host").into_response(); }

    // Accept bearer header (bootstrap + tests) OR session cookie (EventSource + fetch).
    let bearer = req.headers().get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    let cookie = req.headers().get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|c| c.split(';').find_map(|kv| kv.trim().strip_prefix("zf_session=")));
    match bearer.or(cookie) {
        None => (StatusCode::UNAUTHORIZED, "missing token").into_response(),
        Some(t) if t != s.expected_token.as_str() => (StatusCode::FORBIDDEN, "bad token").into_response(),
        Some(_) => next.run(req).await,
    }
}

// Static routes are NOT behind the guard; only the API sub-router is.
let api = Router::new()
    .route("/state", get(get_state))
    .route("/events", get(events))
    .route("/session", post(bootstrap))   // validates bearer, sets cookie
    .route_layer(middleware::from_fn_with_state(state.clone(), guard));

let app = Router::new()
    .nest("/api", api)
    .fallback(static_file)                // public: index.html + assets
    .with_state(state);
```

`bootstrap` validates the bearer token and returns `Set-Cookie: zf_session=<token>; HttpOnly; SameSite=Strict; Path=/`. No `Secure` flag — the transport is plain HTTP on loopback.

**SSE broadcast.** `Sse::new(stream).keep_alive(...)`; `BroadcastStream` from `tokio-stream` wraps `broadcast::Receiver`. Each connection subscribes independently; disconnect drops the receiver automatically. The browser uses native `EventSource('/api/events', { withCredentials: true })` — it cannot set a bearer header, so it authenticates with the session cookie from the bootstrap step.

```rust
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::StreamExt;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use std::{convert::Infallible, time::Duration};

async fn events(State(s): State<AppState>) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let rx = s.tx.subscribe(); // broadcast::Sender<String> in AppState
    let stream = BroadcastStream::new(rx).filter_map(|item| async move {
        match item {
            Ok(payload) => Some(Ok(Event::default().event("update").data(payload))),
            Err(_) => None, // Lagged -> skip
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive"))
}
// Publisher: let _ = state.tx.send(json_string);
```

**Static assets + SPA fallback.** rust-embed 8.14.0 `Metadata` has **no `mimetype()`** — derive Content-Type from the suffix. `Cow<'static,[u8]>` implements `IntoResponse` directly.

```rust
use axum::{http::{header, HeaderValue, StatusCode, Uri}, response::{IntoResponse, Response}};
use rust_embed::{Embed, RustEmbed};

#[derive(Embed)]
#[folder = "dist/"]
struct Assets;

async fn static_file(uri: Uri) -> Response {
    let raw = uri.path().trim_start_matches('/');
    let path = if raw.is_empty() { "index.html" } else { raw };
    if let Some(f) = Assets::get(path) {
        return ([(header::CONTENT_TYPE, HeaderValue::from_static(ct(path)))], f.data).into_response();
    }
    // SPA fallback only for extensionless paths; missing assets -> 404
    if path.contains('.') { return StatusCode::NOT_FOUND.into_response(); }
    match Assets::get("index.html") {
        Some(f) => ([(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))], f.data).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
fn ct(p: &str) -> &'static str {
    match p.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "html" => "text/html; charset=utf-8", "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8", "json" => "application/json",
        "svg" => "image/svg+xml", "png" => "image/png", "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}
```

**Ephemeral port + graceful shutdown.** Bind `127.0.0.1:0`, read `local_addr().port()` **before** moving the listener into `serve`. Trigger shutdown with a `oneshot` fired from the tray/close handler.

```rust
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
let port = listener.local_addr().unwrap().port();
let (tx, rx) = tokio::sync::oneshot::channel::<()>();
axum::serve(listener, app)
    .with_graceful_shutdown(async { let _ = rx.await; })
    .await.unwrap();
```

**Handler tests** use `tower::ServiceExt::oneshot` — no server needed. `Router` implements `tower::Service<Request<Body>>`.

### 1.2 Tokio 1.54.1 + rust-embed 8.14.0

- Tokio 1.54.0 was **yanked** — use 1.54.1. MSRV 1.85 (matches project). Minimal features: `rt-multi-thread`, `macros`, `net`, plus `sync`/`time` for broadcast/keep-alive.
- rust-embed 8.14.0: `#[derive(Embed)] #[folder = "dist/"]`; `Assets::get(path) -> Option<EmbeddedFile>`; debug builds read from disk (use `debug-embed` if you want self-contained debug), release embeds. Windows path normalization is built in — use forward slashes.
- All recommended crates are MIT; no copyleft concerns.

### 1.3 Svelte 5.59.1 + Vite 8.3.8 + Tailwind 4.3.5

**Runes (verified syntax and rules):**
- `$state(v)` — deep reactive proxy for objects/arrays; destructuring loses reactivity.
- `$state.raw(v)` — reassign-only; avoids proxy cost for large data.
- `$state.snapshot(v)` — plain snapshot for APIs/`structuredClone`.
- `$derived(expr)` / `$derived.by(() => ...)` — side-effect-free, lazy.
- `$effect(() => ...)` — **browser only**, external side effects only; **do not write state it reads**; may return teardown.
- `$props()` / `$bindable()` — two-way binding via parent `bind:value`.
- **Module rule:** cannot export a directly reassigned `$state` from `.svelte.ts`. Export a `$state` object and mutate its properties, or expose getter/setter functions.

**Events:** `onclick`, `onpointerdown` (not `on:click`); case-sensitive; `on:` is legacy. TS handlers: `onclick={(e: MouseEvent) => ...}`.

**`vite.config.ts`** (composition of verified pieces):

```ts
import { fileURLToPath, URL } from 'node:url';
import tailwindcss from '@tailwindcss/vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

export default defineConfig({
  base: './',                 // relative asset URLs for random-port serving
  appType: 'spa',
  plugins: [svelte(), tailwindcss()],
  resolve: { alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) } },
});
```

`$lib` is **not** automatic in a Vite-only SPA (that is SvelteKit) — define it as above. `base: './'` makes `index.html` reference `./assets/...`, which is required when Rust serves from an arbitrary path.

**`src/app.css`** (Tailwind 4 CSS-first tokens):

```css
@import "tailwindcss";
@theme {
  --color-panel: oklch(12.9% 0.042 264.695);
  --color-panel-raised: oklch(20.8% 0.042 265.755);
  --color-panel-border: oklch(27.9% 0.041 260.031);
  --color-muted-text: oklch(70.4% 0.04 256.788);
  --color-accent: oklch(76.9% 0.188 70.08);
  --color-accent-strong: oklch(82.8% 0.189 84.429);
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
  --radius-panel: 0.75rem;
}
```

`@theme` tokens generate utilities (`bg-panel`, `text-accent`) **and** CSS variables. In a Svelte `<style>` block using `@apply`, add `@reference "../../app.css";`.

**TypeScript:** official Vite `template-svelte-ts` split — `tsconfig.json` (solution, references), `tsconfig.app.json` (app, `types: ["svelte","vite/client"]`, `$lib` paths), `tsconfig.node.json` (vite.config). `svelte.config.js` can be empty; add `vitePreprocess({ script: true })` only for non-type-only TS. Check script: `svelte-check --tsconfig ./tsconfig.app.json && tsc -p tsconfig.node.json`.

**Build:** `vite build` emits `dist/index.html` + hashed `dist/assets/*`; plain static files, **no Node runtime**. `vite preview` is not a production server.

### 1.4 shadcn-svelte 1.8.1 + Bits UI 2.22.0 + Paneforge 1.0.2 + Lucide

**Setup:** `npx sv add tailwindcss` → add `$lib` aliases → `npx shadcn-svelte@latest init` (base color `slate`, CSS `src/app.css` for Vite) → `npx shadcn-svelte@latest add dialog tabs select switch slider input button badge scroll-area context-menu resizable`. All 11 components exist. shadcn-svelte is copy-paste source wrapping Bits UI primitives — not a runtime package.

**Svelte 5 syntax in components:** `$props()` + `$bindable()`, `Snippet` children rendered with `{@render children?.()}`, `onclick`, `bind:value` / `bind:checked` / `bind:open`. `Select.Root` needs `items={...}` so the label survives menu close.

**Paneforge mapping:** `PaneGroup` → `Resizable.PaneGroup`, `Pane` → `Resizable.Pane`, `PaneResizer` → `Resizable.Handle`. Use `from "paneforge"` (one docs page has a stale `svelte-pane` import). Three-pane layout with collapsible rails:

```svelte
<script lang="ts">
  import * as Resizable from '$lib/components/ui/resizable/index.js';
  import type { Pane } from 'paneforge';
  let left: Pane | undefined = $state();
  let right: Pane | undefined = $state();
  let leftCollapsed = $state(false);
  let rightCollapsed = $state(false);
</script>
<Resizable.PaneGroup direction="horizontal" autoSaveId="zf-layout">
  <Resizable.Pane bind:this={left} defaultSize={18} minSize={10} maxSize={30}
    collapsible={true} collapsedSize={3}
    onCollapse={() => (leftCollapsed = true)} onExpand={() => (leftCollapsed = false)}>
    <AppLibrary />
  </Resizable.Pane>
  <Resizable.Handle withHandle />
  <Resizable.Pane defaultSize={58} minSize={30}><ZoneCanvas /></Resizable.Pane>
  <Resizable.Handle withHandle />
  <Resizable.Pane bind:this={right} defaultSize={24} minSize={15} maxSize={40}
    collapsible={true} collapsedSize={3}
    onCollapse={() => (rightCollapsed = true)} onExpand={() => (rightCollapsed = false)}>
    <Inspector />
  </Resizable.Pane>
</Resizable.PaneGroup>
```

Sizes are percentages. `autoSaveId` persists layout to localStorage. Toggle with `left?.collapse()` / `left?.expand()`.

**ContextMenu** for zone actions: `ContextMenu.Root/Trigger/Content/Group/Item/Separator/Sub/SubTrigger/SubContent/CheckboxItem/Shortcut`; `Item` supports `variant="destructive"` and `disabled`.

**Lucide:** `import Plus from '@lucide/svelte/icons/plus'`; props `size`, `color`, `strokeWidth`; rely on `currentColor` inside shadcn components.

### 1.5 Browser launch + Win32 overlay

**Browser:** `open::that_detached(&url)` — non-blocking, sets `CREATE_NO_WINDOW` on Windows, no features needed. Keep `insecure` **off**. Chromeless `--app=` mode requires `std::process::Command` directly (not an `open` feature) and is browser-specific — UX only, not security.

**Overlay:** a browser page cannot paint over other apps or be click-through — a desktop-level window is required. Use a layered window on a **dedicated thread with its own message loop** (window creation and `GetMessageW` must be on the same thread). Tokio runs on its own separate thread; the main thread only coordinates shutdown.

- Styles: `WS_POPUP | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`; `SetWindowPos(HWND_TOPMOST, SWP_SHOWWINDOW)`. `WS_EX_LAYERED` is required by `UpdateLayeredWindow`.
- Zones need **different alphas** → use `UpdateLayeredWindow` with a 32-bpp DIB section and `BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: 1 }` (`AC_SRC_ALPHA`). **Never mix** with `SetLayeredWindowAttributes` (once called, `UpdateLayeredWindow` fails until the layering bit is cleared/re-set).
- `WNDPROC` signature: `unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT`.
- `HWND` is `!Send`/`!Sync` — create it inside the thread. Marshal zone updates from Tokio via a channel; the overlay thread calls `PostMessageW` locally.
- windows-rs features needed: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Gdi`, `Win32_System_LibraryLoader` (for `GetModuleHandleW`).
- If the overlay thread blocks on handles, use `MsgWaitForMultipleObjectsEx` to avoid the documented deadlock.

---

## 2. Backend implementation tasks

egui dependencies stay until Phase 6. Each task ends with `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.

### Task B1 — App state and command layer
**Files:** new `src/api/state.rs`, `src/api/commands.rs`, `src/api/worker.rs`.

**Blocking-work isolation (critical):** the Win32 engine can block up to **8 seconds** waiting for a launched window (`execute_zone_inner`). An async Axum handler must never call it directly on a Tokio worker — that stalls all requests on that worker. Use a **dedicated command worker thread** with an `mpsc` channel: handlers send a command plus a `oneshot` reply channel and `.await` the reply; the worker executes the synchronous engine calls serially. This also prevents concurrent window moves. (`spawn_blocking` is an alternative, but a single worker better matches the engine's serialized window operations and the app's existing worker-thread pattern.) Measure the channel overhead in §4.

- Define `AppState { inner: Arc<RwLock<Inner>>, tx: broadcast::Sender<String>, cmd_tx: mpsc::Sender<Command>, expected_token: String, allowed_host: String }`.
- `Inner` holds the authoritative draft `AppConfig`, `last_saved_json`, `applied_profile`, `layout_enabled`, `selected_zone`, history, logs.
- Implement command functions that delegate to unchanged backend modules **on the worker thread**: `save`, `apply`, `launch_arrange`, `preview`, `revert`, `disable`, `enable`, profile CRUD, zone mutations, undo/redo. Encode D1–D6: 409 on dirty for apply/launch/load; disabled-mode guards; D3 assignment replacement rules.
- **D6 profile actions are three distinct commands**, not one: `load` (editor rail → draft), `select` (Home → selection only), `apply` (tray → apply when enabled, select-only when disabled). All three take `discard_dirty: bool` (default false) and are dirty-guarded per D2 — including `select`. See architecture plan §5.2. Each gets its own handler and test.
- **Test:** unit tests for each command's state transition, including D1–D6 error cases; a test that a slow command does not block an unrelated API request; the dirty/clean × discard/cancel matrix for `load` / `select` / `apply`.

### Task B2 — HTTP API scaffold
**Files:** new `src/api/mod.rs`, `src/api/routes.rs`.
- Build the router from §5 of the architecture plan; `route_layer` the guard; `.with_state(state)` at top level.
- Bind `127.0.0.1:0`, read port, start `axum::serve` with oneshot graceful shutdown.
- **Test:** `oneshot` tests for every endpoint (status + JSON body), including 401/403/409.

### Task B3 — Auth/Host middleware
**Files:** `src/api/middleware.rs`.
- Implement the §1.1 guard; generate token with `getrandom` (32 bytes, URL-safe); set `Referrer-Policy: no-referrer` and `X-Content-Type-Options: nosniff` on responses.
- **Test:** missing token → 401, bad token → 403, bad Host → 403, valid → 200.

### Task B4 — SSE events
**Files:** `src/api/events.rs`.
- Broadcast `state-changed`, `log`, `status`, `hotkey-action`, `apply-result` events via `broadcast::Sender<String>`; `GET /api/events` per §1.1.
- **Test:** two subscribers both receive a published event; lagged subscriber skips without error.

### Task B5 — Static asset serving
**Files:** `src/web/assets.rs`.
- `#[derive(Embed)] #[folder = "dist/"]`; handler from §1.1 with extension-based Content-Type and SPA fallback.
- **Test:** `/` returns index.html with `text/html`; `/assets/x.js` returns JS content type; missing asset → 404; unknown extensionless path → index.html.

### Task B6 — Browser session management
**Files:** `src/session.rs`.
- On tray "Open editor": `open::that_detached(&format!("http://127.0.0.1:{port}/#token={token}"))`. The token is in the **fragment**, never a query parameter.
- Track sessions; reap stale SSE sessions after 60 s; keep Rust running when browser closes.
- **Test:** session created on launch, reaped after timeout; Rust process unaffected by browser close (manual).

### Task B7 — Native overlay window
**Files:** new `src/overlay_win32.rs`.
- Dedicated thread, layered window, `UpdateLayeredWindow` with per-zone alpha, channel-based updates from Tokio, `PostMessageW` marshalling.
- **Test:** manual Windows test — Ctrl+drag shows correct zone highlight, click-through works, taskbar/Alt-Tab unaffected.

### Task B8 — Lifecycle integration and native message pump
**Files:** modify `src/main.rs`; new `src/native_loop.rs`.

**Windows constraint (critical):** `tray-icon` and `global-hotkey` require a real Win32 message loop **on the same thread where their managers are created**. A Tokio timer is not a substitute — it does not pump `GetMessageW`/`DispatchMessageW`, so tray clicks and hotkey events would never be delivered. The current egui app works only because `eframe`/`winit` owns that loop; the headless app must provide its own.

**Design (matches architecture plan §3.2):**
- A **dedicated native thread** creates the `System` (tray + hotkey managers) and runs the message pump: `GetMessageW` → `TranslateMessage` → `DispatchMessageW` on that same thread. If it blocks on handles, use `MsgWaitForMultipleObjectsEx`.
- **Wake-up:** an `mpsc` send alone does not wake `GetMessageW`. The native thread owns a `Mutex<VecDeque<NativeMessage>>` payload queue; every producer enqueues the payload and then calls `PostThreadMessageW(native_thread_id, WM_APP, 0, 0)`. On `WM_APP` the loop drains the queue. The thread ID is captured with `GetCurrentThreadId()` at startup and shared as a `Send`-safe handle. The native thread never blocks on an 8 s engine call — it sends a command and returns to the loop; the result arrives via this wake-up (this is what keeps the D2 `409` confirmation responsive).
- The **Tokio runtime runs on a separate thread** and owns axum. It never owns the tray/hotkey managers. The **command worker thread** (Task B1) owns authoritative state and runs synchronous engine calls. The main thread only starts and joins threads.
- `System::poll()` is called from the native loop, not a Tokio timer. `Action`s go to the command worker over `mpsc`; API handlers send `Command { payload, reply: oneshot }` to the same worker.
- **Reverse path (API → native):** after any profile CRUD/selection, hotkey edit, settings change or Disable/Enable, the worker sends a `NativeUpdate` over a second `mpsc` channel to the native thread. The native thread rebuilds the tray profile submenu, updates the applied-profile checkmark, toggles the Disable/Enable label and re-registers hotkeys. It never reads shared state directly.
- **Shutdown:** tray "Exit" or close request posts `WM_QUIT` to the native loop and fires the axum `oneshot` shutdown signal. The main thread joins native → worker → Tokio in order; closing each channel terminates its receiver. No detached threads.
- **Runtime choice:** benchmark `current_thread` against `rt-multi-thread` (§4) before locking the feature set. Start with `rt-multi-thread`, then measure.

**Test:** full manual journey — tray-only operation, hotkeys while browser closed, graceful exit; unit tests that (a) the native loop forwards a synthetic `Action` to the command channel, (b) a `NativeUpdate` from an API command rebuilds the tray menu and re-registers hotkeys, and (c) enqueuing a payload plus `PostThreadMessageW(WM_APP)` wakes an idle native loop and the payload is drained. Manual check: rename a profile in the browser and confirm the tray menu updates without any other Windows input.

### Task B9 — Dynamic tray menu and profile workflow
**Files:** modify `src/system.rs` (menu construction), `src/native_loop.rs` (update handling).

The existing `src/system.rs` builds a **fixed** menu (Settings / Re-tile / Reload / Exit). It must become dynamic while staying on the native thread:

- **Header:** "ZoneFlow Studio — Applied: <applied> · Selected: <selected>" — show both dimensions separately.
- **Profiles submenu:** one item per saved profile, with a checkmark on the **applied** profile and a distinct bullet on the **selected** profile when they differ. Rebuild from the current profile list on every `NativeUpdate` (create/rename/delete/select) — never keep a stale menu. When disabled, selection changes Selected only; the tray must show that Applied did not change.
- **Profile click (D6):** when enabled → apply immediately (the `apply` command); when disabled → select-only. Tray, Home and editor-rail actions stay separate and use their own API operations (`apply` / `select` / `load`).
- **Disable/Enable layout (D4):** toggle label and state; suspends all automation; runtime-only.
- **Open editor…** → browser session (Task B6); **Settings…** → open the editor on the Settings view; **Exit** → shutdown (Task B8).

**D2 — dirty-draft confirmation (unified flow, see architecture §5.2):** the native thread cannot read dirty state (the command worker owns it). On a tray profile click it sends the switch command with `discard_dirty=false`. The worker checks dirty state: clean → execute; dirty → return `409 dirty_state` with no state change. On 409 the native thread shows a native `MessageBoxW` ("Unsaved changes in <profile>. Discard them and switch?"): **Discard** resends the same command with `discard_dirty=true` (worker reverts the draft then switches); **Cancel** aborts. The web SPA uses the same contract with a shadcn Dialog on 409. No new infrastructure — just the existing command channel plus the `discard_dirty` flag.

**Test:** menu rebuild after profile create/rename/delete/select; checkmark tracks the applied profile; tray click applies when enabled and selects only when disabled; dirty/clean × discard/cancel matrix for `load`/`select`/`apply`; Disable/Enable label and behavior; Open editor/Settings/Exit.

---

## 3. Frontend implementation tasks

### Task F1 — Project scaffold
**Files:** new `frontend/` with `package.json`, `vite.config.ts`, `tsconfig*.json`, `svelte.config.js`, `src/app.css`, `src/main.ts`, `index.html`.
- Versions from §1.3/§1.4; scripts `dev`, `build`, `check`. `npm run build` emits `dist/`.
- **Check:** `npm run check` clean; `npm run build` produces `dist/index.html` + hashed assets; no Node needed to serve `dist`.

### Task F2 — API client + state store
**Files:** `src/lib/api/client.ts`, `src/lib/api/events.ts`, `src/lib/state/app.svelte.ts`.
- `client.ts`: on load, read the token from `location.hash` and call `POST /api/session` with `Authorization: Bearer <token>` to receive the session cookie; subsequent `fetch` calls are same-origin and rely on the cookie (bearer header optional). Typed request/response interfaces mirroring §5 contracts, including the `{ error: { code, message } }` error shape.
- `events.ts`: `new EventSource('/api/events', { withCredentials: true })` with reconnect; the cookie authenticates it. Dispatch events into the store.
- `app.svelte.ts`: exported `$state` object (mutate properties only), `$derived` dirty flag via `$state.snapshot` comparison, getter/setter functions for reassignment.
- **Check:** store updates reactively; dirty flag correct; SSE reconnect works.

### Task F3 — Shell + panels
**Files:** `App.svelte`, `components/TopBar.svelte`, `StatusBar.svelte`, `LeftRail/*`, `Inspector/*`.
- Three-pane `Resizable.PaneGroup` from §1.4; four independent status dimensions (Saved/Applied/Enabled/Selected) with distinct indicators; contextual inspector.
- **Check:** no clipping at 1280×720 / 1920×1080 / maximized; rails collapsible; inspector contextual.

### Task F4 — SVG canvas
**Files:** `components/Canvas/ZoneCanvas.svelte`, `ZoneRect.svelte`, `SnapGuides.svelte`, `DragPreview.svelte`.
- `viewBox` from work-area pixels; zone % → SVG coords; pointer events with `setPointerCapture`; commit on `pointerup` only (no HTTP per move); snap guides at 25/50/75; DnD assignment; context menu; undo/redo via API.
- **Check:** create/resize/split/merge/draw/DnD/context-menu/undo all work; geometry matches `engine_core`.

### Task F5 — Dialogs + inspector
**Files:** `components/dialogs/*`, `components/Inspector/*`.
- Wizard (template/grid/single/draw + live preview), picker (Installed/Running/Browse), confirmations (D2/D3), close dialog; inspector Process/Browser/Macro tabs + console.
- **Check:** full user journey; D1–D6 behaviors; no data loss.

### Task F6 — Settings/hotkeys views
**Files:** `components/views/HotkeysView.svelte`, `SettingsView.svelte`.
- Editable bindings and settings via API; autostart toggle; close behavior.
- **Check:** changes persist; hotkeys re-register; autostart syncs.

---

## 4. Testing strategy

- **Backend unit/integration:** all 43 existing tests must keep passing; new `oneshot` API tests for every endpoint and D1–D6 error case; command-layer state-transition tests.
- **Frontend:** `svelte-check` + `tsc` clean; component tests for store reactivity and dirty flag (Vitest if added — justify the dependency before adding).
- **Manual Windows journeys:** tray-only operation; hotkeys with browser closed; browser open/close lifecycle; overlay Ctrl+drag; full edit→save→apply→launch→tray→disable→enable journey; unsaved-change prompts including the tray Discard/Cancel dialog.
- **Real editing functional test (dev-only):** browser automation (e.g. Playwright, test-only dependency) performs a real SVG zone-boundary drag, asserts the frontend interaction completes and the backend persists the geometry. This is a functional test, not the `PointerActivity` benchmark.
- **Performance:** three workloads from `docs/performance/results.md`; measure Rust and browser process trees separately (including browser CPU for Editor open); ≥3 runs, report ranges. **Benchmark Tokio `current_thread` against `rt-multi-thread`** before locking the runtime feature set — the app is I/O-light and `current_thread` may reduce RAM. Also measure the command-worker channel overhead.
- **Paneforge isolation:** keep all Paneforge usage behind the shadcn-svelte `Resizable` component wrapper (`$lib/components/ui/resizable`). Do not import `paneforge` directly in feature components — its stale maintenance status makes it a replaceable dependency.

---

## 5. Checkpoints

| Checkpoint | After | Gate |
|---|---|---|
| C0 | Research (this doc) | APIs verified; plan approved; no implementation started |
| C1 | B1–B2 | API tests pass; egui UI still builds |
| C2 | B3–B6 | Auth/SSE/static/session tests pass; browser loads from embedded assets |
| C3 | B7–B9 | Overlay works; native thread owns tray/hotkeys; dynamic tray menu + D2 dialog verified; lifecycle joins all threads |
| C4 | F1–F2 | `npm run check`/`build` clean; store + SSE work; auth bootstrap + failure paths verified |
| C5 | F3–F4 | Shell + canvas parity; no clipping; geometry matches Rust; real drag functional test passes |
| C6 | F5–F6 | Full journey; D1–D6 verified; no data loss |
| C7 | Phase 6 of architecture plan | Three workloads measured; egui removed; docs synced |

A failing gate blocks the next task. No workstream begins before its predecessor's gate passes.
