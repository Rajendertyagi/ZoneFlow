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
use crate::system::HotkeyFailures;
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
    hotkey_failures: HotkeyFailures,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("zoneflow-worker".into())
        .spawn(move || {
            let mut inner = Inner::new(cfg);
            let mut running_cache = None;
            let notifier = Arc::new(notifier);

            while let Ok(cmd) = rx.recv() {
                // Capture before `cmd.payload` is moved into `handle`. Read-only
                // commands (e.g. StateGet) must not broadcast `state-changed`,
                // otherwise a client refreshing on that event feeds back into
                // another event forever (idle-CPU loop).
                let emits_state_change = cmd.payload.emits_state_change();
                let mut ctx = CommandContext {
                    notifier: notifier.clone(),
                    shutdown: shutdown.clone(),
                    editor_url: editor_url.clone(),
                    running_cache: &mut running_cache,
                    hotkey_failures: hotkey_failures.clone(),
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
                    // Same source as the MoveWindow drag-release handler so the
                    // overlay always shows exactly where a drop will snap.
                    notifier.set_zones(inner.drag_zones());

                    if emits_state_change {
                        let event = serde_json::json!({
                            "type": "state-changed",
                            "payload": {},
                        });
                        if let Err(e) = sse_tx.send(event.to_string()) {
                            tracing::debug!(error = %e, "no SSE subscribers for state-changed event");
                        }
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
            Arc::new(crate::browser::EDITOR_URL.to_string()),
            Arc::new(std::sync::Mutex::new(Vec::new())),
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

    /// A read-only `StateGet` must produce no SSE frame, while a mutation
    /// (`Disable`) produces exactly one. A trailing `StateGet` after each
    /// command acts as a deterministic barrier: the worker processes commands
    /// in order, so receiving that barrier's reply proves the preceding
    /// command's loop body (including the emission decision) has completed —
    /// no sleeps or yields needed.
    #[test]
    fn read_only_commands_emit_no_sse_event_but_mutations_do() {
        let (tx, rx) = mpsc::channel::<Command>();
        let (sse_tx, mut sse_rx) = broadcast::channel(16);
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
            Arc::new(crate::browser::EDITOR_URL.to_string()),
            Arc::new(std::sync::Mutex::new(Vec::new())),
        );

        fn send(tx: &mpsc::Sender<Command>, payload: CommandPayload) -> Result<Value, ApiError> {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(Command {
                payload,
                reply: reply_tx,
            })
            .unwrap();
            reply_rx.blocking_recv().unwrap()
        }

        // Two read-only commands; the second is the barrier.
        send(&tx, CommandPayload::StateGet).unwrap();
        send(&tx, CommandPayload::StateGet).unwrap();
        assert!(
            matches!(
                sse_rx.try_recv(),
                Err(broadcast::error::TryRecvError::Empty)
            ),
            "StateGet must not emit a state-changed event"
        );

        // A mutation emits exactly one event; a trailing read-only command is
        // the barrier proving the emission has already happened.
        send(&tx, CommandPayload::Disable).unwrap();
        send(&tx, CommandPayload::StateGet).unwrap();
        let frame = sse_rx
            .try_recv()
            .expect("Disable must emit one state-changed event");
        let parsed: Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(parsed["type"], "state-changed");
        assert!(
            matches!(
                sse_rx.try_recv(),
                Err(broadcast::error::TryRecvError::Empty)
            ),
            "Disable must emit exactly one event"
        );

        // The native completion ack (U-03) must emit exactly one state-changed
        // frame so clients refresh the shared failure slot *after* registration
        // finishes. A trailing StateGet is the barrier proving the emission has
        // happened. If the ack were misclassified as read-only, try_recv would
        // return Empty and this test would fail.
        send(&tx, CommandPayload::HotkeyRegistrationFinished).unwrap();
        send(&tx, CommandPayload::StateGet).unwrap();
        let frame = sse_rx.try_recv().expect(
            "HotkeyRegistrationFinished must emit one post-registration state-changed event",
        );
        let parsed: Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(parsed["type"], "state-changed");
        assert!(
            matches!(
                sse_rx.try_recv(),
                Err(broadcast::error::TryRecvError::Empty)
            ),
            "HotkeyRegistrationFinished must emit exactly one event"
        );

        drop(tx);
        worker.join().unwrap();
    }
}
