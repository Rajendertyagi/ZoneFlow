//! Loopback-only auth guard and response hardening headers.
//!
//! Accepts either a bearer token (bootstrap / tests) or a `zf_session` cookie
//! (fetch / EventSource). Validates `Host` exactly to mitigate DNS rebinding.
//! Applied to the `/api` sub-router only — static assets stay public.

use super::{ApiError, AppState};
use axum::body::Body;
use axum::extract::State;
use axum::http::header::{AUTHORIZATION, HOST, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS};
use axum::http::{HeaderMap, HeaderValue, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Read a cookie value from the `Cookie` header without a cookie crate.
pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.trim().to_string())
}

/// Build the `Set-Cookie` header used by `POST /api/session`.
pub fn session_cookie(session_id: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "zf_session={session_id}; Path=/; HttpOnly; SameSite=Strict"
    ))
    .expect("session id is URL-safe ASCII")
}

/// Guard: validate Host, then require a valid bearer token or session cookie.
pub async fn auth_guard(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // 1) Host must be exactly 127.0.0.1:<bound port>.
    let expected_host = format!("127.0.0.1:{}", state.port);
    let host_ok = request
        .headers()
        .get(HOST)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|h| h == expected_host);
    if !host_ok {
        return ApiError::Forbidden.into_response();
    }

    // 2) Bearer (bootstrap) OR session cookie (fetch / EventSource).
    let bearer = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
        .trim();
    let cookie = cookie_value(request.headers(), "zf_session");

    let bearer_ok = bearer == state.token.as_str();
    let cookie_ok = cookie
        .as_deref()
        .is_some_and(|id| state.sessions.validate(id));

    if !bearer_ok && !cookie_ok {
        // No credentials at all → 401; wrong credentials → 403.
        if bearer.is_empty() && cookie.is_none() {
            return ApiError::Unauthorized.into_response();
        }
        return ApiError::Forbidden.into_response();
    }

    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    response
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::worker::Command;
    use crate::api::AppState;
    use crate::native_loop::{HwndSend, NativeQueue, SharedEvent, ShutdownHandle};
    use crate::session::SessionRegistry;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware::from_fn_with_state;
    use axum::routing::get;
    use axum::Router;
    use std::sync::{mpsc, Arc};
    use tokio::sync::{broadcast, watch};
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<Command>();
        let (sse_tx, _sse_rx) = broadcast::channel(4);
        let (_stop_tx, stop_rx) = watch::channel(false);
        let queue = Arc::new(NativeQueue::new());
        let shutdown = ShutdownHandle::new(
            queue,
            SharedEvent::new_test(),
            Arc::new(std::sync::OnceLock::<HwndSend>::new()),
            tokio::sync::watch::Sender::new(false),
            None,
        );
        AppState {
            cmd_tx,
            sse_tx,
            stop_rx,
            token: Arc::new("test-token".into()),
            port: 12345,
            sessions: Arc::new(SessionRegistry::new()),
            shutdown,
        }
    }

    fn test_router(state: AppState) -> Router {
        Router::new()
            .route("/api/ping", get(|| async { "pong" }))
            .route_layer(from_fn_with_state(state.clone(), auth_guard))
            .with_state(state)
    }

    async fn call(router: &Router, headers: &[(&str, &str)]) -> Response {
        let mut builder = Request::builder().uri("/api/ping");
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        router
            .clone()
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn missing_token_is_401() {
        let res = call(&test_router(test_state()), &[("host", "127.0.0.1:12345")]).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bad_token_is_403() {
        let res = call(
            &test_router(test_state()),
            &[
                ("host", "127.0.0.1:12345"),
                ("authorization", "Bearer wrong"),
            ],
        )
        .await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn bad_host_is_403_even_with_valid_token() {
        let res = call(
            &test_router(test_state()),
            &[
                ("host", "evil.example:12345"),
                ("authorization", "Bearer test-token"),
            ],
        )
        .await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn valid_bearer_is_200_with_hardening_headers() {
        let res = call(
            &test_router(test_state()),
            &[
                ("host", "127.0.0.1:12345"),
                ("authorization", "Bearer test-token"),
            ],
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()["referrer-policy"], "no-referrer");
        assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    }

    #[tokio::test]
    async fn valid_session_cookie_is_200() {
        let state = test_state();
        let id = state.sessions.create();
        let cookie = format!("zf_session={id}");
        let res = call(
            &test_router(state),
            &[("host", "127.0.0.1:12345"), ("cookie", &cookie)],
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[test]
    fn cookie_value_parses_named_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            HeaderValue::from_static("a=1; zf_session=abc123; b=2"),
        );
        assert_eq!(
            cookie_value(&headers, "zf_session").as_deref(),
            Some("abc123")
        );
        assert_eq!(cookie_value(&headers, "missing"), None);
    }

    #[test]
    fn session_cookie_has_httponly_and_strict() {
        let v = session_cookie("abc123");
        let s = v.to_str().unwrap();
        assert!(s.contains("HttpOnly"));
        assert!(s.contains("SameSite=Strict"));
        assert!(s.contains("Path=/"));
    }

    #[tokio::test]
    async fn error_envelope_is_json() {
        let res = call(&test_router(test_state()), &[("host", "127.0.0.1:12345")]).await;
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "unauthorized");
    }
}
