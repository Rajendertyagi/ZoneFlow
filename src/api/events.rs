//! Server-Sent Events transport (backend → UI).
//!
//! Each handler subscribes to the worker's broadcast channel and selects on a
//! `watch` shutdown receiver via `take_until`, so shutdown can end every SSE
//! response before Axum's graceful-shutdown await (architecture §3.6 step 2).

use super::AppState;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::{Stream, StreamExt};
use std::convert::Infallible;
use tokio::sync::{broadcast, watch};
use tokio_stream::wrappers::BroadcastStream;

/// Build the event stream. A lagged subscriber simply skips missed events
/// (state is a full refresh, so replaying old events would be wrong).
pub fn sse_stream(
    mut stop: watch::Receiver<bool>,
    rx: broadcast::Receiver<String>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let events = BroadcastStream::new(rx).filter_map(|item| async move {
        match item {
            Ok(payload) => Some(Ok(Event::default().data(payload))),
            Err(e) => {
                tracing::debug!(error = %e, "SSE subscriber lagged; skipping missed events");
                None
            }
        }
    });
    // `changed()` also returns Err when the sender is dropped — either way the
    // stream must end so graceful shutdown cannot hang.
    let stopper = async move {
        let _ = stop.changed().await;
    };
    events.take_until(stopper)
}

/// `GET /api/events` handler.
pub async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_tx.subscribe();
    Sse::new(sse_stream(state.stop_rx, rx)).keep_alive(KeepAlive::default())
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use futures_util::StreamExt;
    use std::time::Duration;

    #[tokio::test]
    async fn stream_ends_on_watch_signal() {
        let (tx, rx) = broadcast::channel(8);
        let (stop_tx, stop_rx) = watch::channel(false);
        let mut stream = Box::pin(sse_stream(stop_rx, rx));

        tx.send("hello".to_string()).unwrap();
        let first = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(format!("{first:?}").contains("hello"));

        // send_replace does not fail with zero receivers (unlike `send`).
        stop_tx.send_replace(true);
        let ended = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap();
        assert!(
            ended.is_none(),
            "stream should end when shutdown is signalled"
        );
    }

    #[tokio::test]
    async fn stream_ends_when_sender_dropped() {
        let (tx, rx) = broadcast::channel(8);
        let (_stop_tx, stop_rx) = watch::channel(false);
        let mut stream = Box::pin(sse_stream(stop_rx, rx));
        drop(tx);
        // With no events and the watch sender still alive, the stream stays
        // open; dropping the watch sender must end it.
        drop(_stop_tx);
        let ended = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap();
        assert!(ended.is_none());
    }

    /// Verifies the shutdown invariant from architecture §3.6: an endless SSE
    /// stream must end on the watch signal so `axum::serve`'s graceful-shutdown
    /// await cannot hang. Uses a real loopback listener and a raw HTTP client.
    #[tokio::test]
    async fn graceful_shutdown_returns_after_sse_stream_ends() {
        use axum::routing::get;
        use tokio::io::Interest;
        use tokio::sync::oneshot;

        let (sse_tx, _rx) = broadcast::channel(8);
        let (stop_tx, stop_rx) = watch::channel(false);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let app = Router::new().route(
            "/events",
            get({
                let tx = sse_tx.clone();
                move || {
                    let rx = tx.subscribe();
                    let stop = stop_rx.clone();
                    async move { Sse::new(sse_stream(stop, rx)) }
                }
            }),
        );

        // Test-only ephemeral listener. The application itself always binds
        // the fixed port defined in `browser::EDITOR_PORT` (4000).
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
        socket.ready(Interest::WRITABLE).await.unwrap();
        let request = b"GET /events HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: keep-alive\r\n\r\n";
        let written = socket.try_write(request).unwrap();
        assert_eq!(written, request.len());
        let mut buf = [0u8; 256];
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            socket.ready(Interest::READABLE),
        )
        .await
        .unwrap()
        .unwrap();
        let n = socket.try_read(&mut buf).unwrap();
        assert!(n > 0, "SSE response headers should arrive");

        // Close SSE first, then signal Axum — the exact §3.6 step 2 → step 3 order.
        stop_tx.send_replace(true);
        shutdown_tx.send(()).unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(3), server)
            .await
            .expect("graceful shutdown hung: SSE stream did not end")
            .unwrap();
    }

    #[tokio::test]
    async fn two_subscribers_each_receive_the_event() {
        let (tx, _rx) = broadcast::channel(8);
        let (_stop_tx, stop_rx) = watch::channel(false);
        let mut a = Box::pin(sse_stream(stop_rx.clone(), tx.subscribe()));
        let mut b = Box::pin(sse_stream(stop_rx, tx.subscribe()));
        tx.send("broadcast".to_string()).unwrap();
        let ea = tokio::time::timeout(Duration::from_secs(1), a.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let eb = tokio::time::timeout(Duration::from_secs(1), b.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(format!("{ea:?}").contains("broadcast"));
        assert!(format!("{eb:?}").contains("broadcast"));
    }
}
