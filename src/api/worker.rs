//! Dedicated command worker thread (thread 2 of the architecture).
//!
//! Owns the authoritative [`Inner`] state and executes synchronous engine
//! calls (including the 8-second launch wait). Never touches Win32 windows or
//! tray/hotkey objects, and never blocks a Tokio worker.

use super::commands::{handle, CommandContext, CommandPayload};
use super::state::Inner;
use super::ApiError;
use crate::config::AppConfig;
use crate::native_loop::{NativeNotifier, ShutdownHandle};
use serde_json::Value;
use std::sync::mpsc;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot};

/// One command plus its oneshot reply channel.
pub struct Command {
    pub payload: CommandPayload,
    pub reply: oneshot::Sender<Result<Value, ApiError>>,
}

/// Spawn the worker. It exits when every `Command` sender is dropped (main
/// drops its clone at shutdown step 7, after all HTTP handlers have finished).
pub fn spawn_worker(
    rx: mpsc::Receiver<Command>,
    cfg: AppConfig,
    sse_tx: broadcast::Sender<String>,
    notifier: NativeNotifier,
    shutdown: ShutdownHandle,
    editor_url: Arc<String>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("zoneflow-worker".into())
        .spawn(move || {
            let mut inner = Inner::new(cfg);
            let mut running_cache = None;
            let notifier = Arc::new(notifier);

            while let Ok(cmd) = rx.recv() {
                let mut ctx = CommandContext {
                    notifier: notifier.clone(),
                    shutdown: shutdown.clone(),
                    editor_url: editor_url.clone(),
                    running_cache: &mut running_cache,
                };
                let result = handle(cmd.payload, &mut inner, &mut ctx);
                let succeeded = result.is_ok();
                // A dropped receiver means no browser is connected — normal.
                if cmd.reply.send(result).is_err() {
                    tracing::debug!("command reply receiver was dropped");
                }
                if succeeded {
                    // Keep the native overlay's cached geometry in sync with
                    // the authoritative draft (reverse update, §3.3 step 4).
                    let area = crate::engine::work_area();
                    let zones = crate::engine_core::compute_zones(
                        inner.draft.active(),
                        area,
                        inner.draft.gap,
                    );
                    notifier.set_zones(zones);

                    let event = serde_json::json!({
                        "type": "state-changed",
                        "payload": {},
                    });
                    if let Err(e) = sse_tx.send(event.to_string()) {
                        tracing::debug!(error = %e, "no SSE subscribers for state-changed event");
                    }
                }
            }

            tracing::info!("command worker exiting");
            // Dropping `notifier` and `sse_tx` here stops all producers into the
            // native queue before main performs its final drain (§3.6 step 7).
            drop(notifier);
            drop(sse_tx);
        })
        .expect("failed to spawn command worker thread")
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_config;
    use crate::native_loop::{HwndSend, NativeQueue, SharedEvent};

    #[test]
    fn worker_replies_and_exits_when_senders_drop() {
        let (tx, rx) = mpsc::channel::<Command>();
        let (sse_tx, _sse_rx) = broadcast::channel(16);
        let queue = Arc::new(NativeQueue::new());
        let notifier = NativeNotifier::new(queue.clone(), Arc::new(std::sync::OnceLock::new()));
        let shutdown = ShutdownHandle::new(
            queue,
            SharedEvent::new_test(),
            Arc::new(std::sync::OnceLock::<HwndSend>::new()),
            tokio::sync::watch::Sender::new(false),
            None,
        );
        let worker = spawn_worker(
            rx,
            default_config(),
            sse_tx,
            notifier,
            shutdown,
            Arc::new("http://127.0.0.1:1/#token=x".into()),
        );

        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(Command {
            payload: CommandPayload::StatusGet,
            reply: reply_tx,
        })
        .unwrap();
        let value = reply_rx.blocking_recv().unwrap().unwrap();
        assert!(value["enabled"].as_bool().unwrap());

        drop(tx);
        worker.join().unwrap();
    }
}
