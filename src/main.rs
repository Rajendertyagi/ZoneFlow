#![windows_subsystem = "windows"]

// The implementation lives in the `screenlayout` library crate (src/lib.rs).
// Importing the modules from there keeps this binary as a thin entry point and
// lets integration tests link against the real types. Runtime behaviour is
// unchanged: the same modules, threads and code paths are used as before.
use screenlayout::{api, browser, config, native_loop, system};

use api::AppState;
use native_loop::{
    HwndSend, NativeDirtyConfirm, NativeLoop, NativeNotifier, NativeQueue, SharedEvent,
    ShutdownHandle,
};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use system::HotkeyFailures;
use tokio::sync::{broadcast, oneshot, watch};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Threading::CreateEventW;
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MESSAGEBOX_STYLE,
};

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

    // --- channels (architecture §3.2 step 1) ---
    let (cmd_tx, cmd_rx) = mpsc::channel::<api::worker::Command>();
    // The HTTP thread reports `Ok(())` once the fixed port is bound, or an
    // `Err(message)` describing why it could not bind. There is no fallback.
    let (bind_tx, bind_rx) = oneshot::channel::<Result<(), String>>();
    let (axum_shutdown_tx, axum_shutdown_rx) = oneshot::channel::<()>();
    let (sse_tx, _sse_rx) = broadcast::channel::<String>(256);
    let (stop_tx, stop_rx) = watch::channel(false);

    let queue = Arc::new(NativeQueue::new());
    let event = create_shutdown_event();
    // Shared hotkey-registration failure slot: written on the native thread by
    // `System`, read by the command worker into `StateSnapshot` (Finding U-03).
    let hotkey_failures: HotkeyFailures = Arc::new(Mutex::new(Vec::new()));
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
    let server_cmd_tx = cmd_tx.clone();
    let server_sse_tx = sse_tx.clone();
    let server_shutdown = shutdown.clone();
    let server =
        std::thread::Builder::new()
            .name("zoneflow-http".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build current_thread Tokio runtime");
                runtime.block_on(async move {
                    let listener =
                        match tokio::net::TcpListener::bind(("127.0.0.1", browser::EDITOR_PORT))
                            .await
                        {
                            Ok(listener) => listener,
                            Err(e) => {
                                let message =
                                    format!("cannot bind 127.0.0.1:{}: {e}", browser::EDITOR_PORT);
                                let _ = bind_tx.send(Err(message));
                                return;
                            }
                        };
                    // Publish readiness before awaiting serve. The port is fixed,
                    // so no address needs to be sent back to main.
                    let _ = bind_tx.send(Ok(()));

                    let state = AppState {
                        cmd_tx: server_cmd_tx,
                        sse_tx: server_sse_tx,
                        stop_rx,
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

    // Main blocks until the fixed port is bound (or binding fails). A failure
    // is fatal: never start an inaccessible instance on a random port.
    match bind_rx.blocking_recv() {
        Ok(Ok(())) => {}
        Ok(Err(message)) => {
            tracing::error!("ZoneFlow startup failed: {message}");
            tracing::error!(
                "Port {} is required; refusing to fall back to a random port.",
                browser::EDITOR_PORT
            );
            // The GUI subsystem hides the console, so a file-log-only error is
            // invisible to the user. Surface the fatal bind failure in a modal
            // dialog. `None` is the owner on purpose: no native window exists
            // yet at this point (same pattern as the D2 dialog in native_loop).
            let text = format!(
                "ZoneFlow could not start.\n\nPort {} is required and there is no random-port fallback. Free that port and try again.\n\nDetails: {message}",
                browser::EDITOR_PORT
            );
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            wide.push(0);
            // SAFETY: `wide` is a local, null-terminated UTF-16 buffer that lives
            // for the whole call; MessageBoxW copies its text. No native window
            // exists yet, so a NULL owner is used deliberately.
            unsafe {
                MessageBoxW(
                    None,
                    PCWSTR::from_raw(wide.as_ptr()),
                    w!("ZoneFlow — startup failed"),
                    MESSAGEBOX_STYLE(MB_ICONERROR.0 | MB_OK.0 | MB_SETFOREGROUND.0),
                );
            }
            let _ = server.join();
            std::process::exit(1);
        }
        Err(_) => {
            tracing::error!("HTTP thread did not report its bind result");
            let _ = server.join();
            std::process::exit(1);
        }
    }
    tracing::info!(
        port = browser::EDITOR_PORT,
        "HTTP server bound on fixed port"
    );

    // --- thread 2: command worker (spawned after the bind succeeds) ---
    let editor_url = Arc::new(browser::EDITOR_URL.to_string());
    let worker = api::worker::spawn_worker(
        cmd_rx,
        cfg.clone(),
        sse_tx.clone(),
        notifier,
        shutdown.clone(),
        editor_url,
        hotkey_failures.clone(),
    );

    // --- thread 1 (main): native objects + message loop (§3.2 step 4) ---
    let mut native = match NativeLoop::new(&cfg, queue.clone(), event, hotkey_failures) {
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
    native.run(cmd_tx.clone(), &shutdown, &NativeDirtyConfirm);

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
