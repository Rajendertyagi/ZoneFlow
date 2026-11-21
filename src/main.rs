mod api;
mod apps;
mod autostart;
mod config;
mod editor;
mod engine;
mod engine_core;
mod native_loop;
mod overlay;
mod session;
mod system;
mod web;

use api::AppState;
use native_loop::{HwndSend, NativeLoop, NativeNotifier, NativeQueue, SharedEvent, ShutdownHandle};
use std::sync::{mpsc, Arc, OnceLock};
use tokio::sync::{broadcast, oneshot, watch};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Threading::CreateEventW;

/// Sent by the Tokio thread to main once the listener is bound.
struct Bound {
    port: u16,
    token: String,
}

fn init_logging() {
    // Logging to a file next to the exe; level via RUST_LOG, default "info".
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("zoneflow.log")));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_ansi(false)
        .with_writer(move || -> Box<dyn std::io::Write> {
            if let Some(ref p) = log_path {
                if let Ok(f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(p)
                {
                    return Box::new(f);
                }
            }
            Box::new(std::io::sink())
        })
        .finish();
    // Only fails if a global subscriber was already installed; harmless.
    let _ = tracing::subscriber::set_global_default(subscriber);
}

/// Create the manual-reset shutdown event (§3.6 step 4 guaranteed path).
fn create_shutdown_event() -> HANDLE {
    // SAFETY: CreateEventW is called with no security attributes, manual-reset
    // = true, initial state = false and no name. It returns a valid handle or
    // an error; no caller pointers are involved.
    unsafe { CreateEventW(None, true, false, None).expect("CreateEventW failed") }
}

fn main() {
    init_logging();

    let cfg = config::load_config();
    let token = session::generate_token();

    // --- channels (architecture §3.2 step 1) ---
    let (cmd_tx, cmd_rx) = mpsc::channel::<api::worker::Command>();
    let (bound_tx, bound_rx) = oneshot::channel::<Bound>();
    let (axum_shutdown_tx, axum_shutdown_rx) = oneshot::channel::<()>();
    let (sse_tx, _sse_rx) = broadcast::channel::<String>(256);
    let (stop_tx, stop_rx) = watch::channel(false);

    let queue = Arc::new(NativeQueue::new());
    let event = create_shutdown_event();
    let hwnd_slot = Arc::new(OnceLock::<HwndSend>::new());
    let notifier = NativeNotifier::new(queue.clone(), hwnd_slot.clone());
    let shutdown = ShutdownHandle::new(
        queue.clone(),
        SharedEvent(event),
        hwnd_slot.clone(),
        stop_tx,
        Some(axum_shutdown_tx),
    );

    // --- thread 3: Tokio current_thread runtime (§3.2 step 3) ---
    let server_token = token.clone();
    let server_cmd_tx = cmd_tx.clone();
    let server_sse_tx = sse_tx.clone();
    let server_shutdown = shutdown.clone();
    let server = std::thread::Builder::new()
        .name("zoneflow-http".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build current_thread Tokio runtime");
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                    .await
                    .expect("failed to bind 127.0.0.1:0");
                let port = listener.local_addr().expect("local_addr failed").port();
                // Publish the bound port + token before awaiting serve.
                let _ = bound_tx.send(Bound {
                    port,
                    token: server_token.clone(),
                });

                let sessions = Arc::new(session::SessionRegistry::new());
                session::spawn_reaper(sessions.clone());
                let state = AppState {
                    cmd_tx: server_cmd_tx,
                    sse_tx: server_sse_tx,
                    stop_rx,
                    token: Arc::new(server_token),
                    port,
                    sessions,
                    shutdown: server_shutdown,
                };
                let app = api::build_router(state);
                if let Err(e) = axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        let _ = axum_shutdown_rx.await;
                    })
                    .await
                {
                    tracing::warn!(error = %e, "axum server stopped with an error");
                }
            });
        })
        .expect("failed to spawn HTTP thread");

    // Main blocks on the bound oneshot (bounded by the thread's own startup).
    let bound = bound_rx.blocking_recv().expect("HTTP thread did not bind");
    let editor_url = Arc::new(format!(
        "http://127.0.0.1:{}/#token={}",
        bound.port, bound.token
    ));
    tracing::info!(port = bound.port, "HTTP server bound");

    // --- thread 2: command worker (§3.2 step 2, spawned after bind so it can
    // receive the editor URL) ---
    let worker = api::worker::spawn_worker(
        cmd_rx,
        cfg.clone(),
        sse_tx.clone(),
        notifier,
        shutdown.clone(),
        editor_url,
    );

    // --- thread 1 (main): native objects + message loop (§3.2 step 4) ---
    let mut native = match NativeLoop::new(&cfg, queue.clone(), event) {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(error = %e, "native initialization failed; shutting down");
            shutdown.trigger();
            let _ = server.join();
            drop(cmd_tx);
            let _ = worker.join();
            std::process::exit(1);
        }
    };
    native.publish_hwnd(&hwnd_slot);

    // Enter the message loop. Tray Exit and POST /api/shutdown both converge
    // on `shutdown.trigger()`, which sets the event and posts WM_STOP.
    native.run(cmd_tx.clone(), &shutdown);

    // --- ordered shutdown, steps 6–13 (architecture §3.6) ---
    // 6) Join Tokio. SSE streams were closed at step 2, so this cannot hang.
    if let Err(e) = server.join() {
        tracing::warn!(error = ?e, "HTTP thread panicked during join");
    }

    // 7) Drop main's cmd_tx clone; the worker drains queued commands and exits.
    drop(cmd_tx);
    if let Err(e) = worker.join() {
        tracing::warn!(error = ?e, "command worker panicked during join");
    }

    // 8–12) Final drain, kill timer, destroy windows, drop System, close event.
    native.finish(&queue);

    tracing::info!("ZoneFlow shutdown complete");
}
