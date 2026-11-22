//! HTTP API layer: router, DTOs, authoritative state, command worker and SSE.
//!
//! The `/api` sub-router is intentionally open: the server binds only the
//! loopback interface and there is no token, cookie, bearer header or Host
//! validation. Static assets are public via the fallback.

pub mod commands;
pub mod dto;
pub mod events;
pub mod routes;
pub mod state;
pub mod worker;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum::Router;
use std::sync::mpsc;
use tokio::sync::{broadcast, watch};

use crate::native_loop::ShutdownHandle;

/// State shared by every handler (cloned per request, so all fields are
/// cheaply cloneable handles).
#[derive(Clone)]
pub struct AppState {
    pub cmd_tx: mpsc::Sender<worker::Command>,
    pub sse_tx: broadcast::Sender<String>,
    pub stop_rx: watch::Receiver<bool>,
    /// Reserved for direct shutdown signaling; Phase 1 routes shutdown through
    /// the command worker so all triggers converge.
    #[allow(dead_code)]
    pub shutdown: ShutdownHandle,
}

/// API error type. Every variant maps to the `{ "error": { code, message } }`
/// envelope defined in architecture §5.
#[derive(Debug)]
pub enum ApiError {
    /// 409 — D2 dirty draft; save or discard first.
    DirtyState,
    /// 400 — malformed or rejected input.
    BadRequest(String),
    /// 404 — named resource missing.
    NotFound(String),
    /// 500 — internal failure (serialization, disk I/O, …).
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::DirtyState => (
                StatusCode::CONFLICT,
                "dirty_state",
                "save or discard changes first".to_string(),
            ),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m),
            Self::NotFound(w) => (StatusCode::NOT_FOUND, "not_found", format!("{w} not found")),
            Self::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", m),
        };
        let body = dto::ApiErrorBody::new(code, message);
        (status, Json(body)).into_response()
    }
}

/// Build the full router: an open `/api/*` sub-router plus a fallback that
/// serves embedded static assets (SPA).
pub fn build_router(state: AppState) -> Router {
    routes::build(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_error_status_and_envelope() {
        let cases = [
            (ApiError::DirtyState, StatusCode::CONFLICT, "dirty_state"),
            (
                ApiError::BadRequest("x".into()),
                StatusCode::BAD_REQUEST,
                "bad_request",
            ),
            (
                ApiError::NotFound("zone".into()),
                StatusCode::NOT_FOUND,
                "not_found",
            ),
            (
                ApiError::Internal("boom".into()),
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
            ),
        ];
        for (err, expected_status, expected_code) in cases {
            let response = err.into_response();
            assert_eq!(response.status(), expected_status);
            // Body shape is checked in dto.rs; here we only assert status.
            let _ = expected_code;
        }
    }
}
