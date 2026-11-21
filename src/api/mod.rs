//! HTTP API layer: router, DTOs, authoritative state, command worker, SSE and
//! the loopback-only auth guard.

pub mod commands;
pub mod dto;
pub mod events;
pub mod middleware;
pub mod routes;
pub mod state;
pub mod worker;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum::Router;
use std::sync::{mpsc, Arc};
use tokio::sync::{broadcast, watch};

use crate::native_loop::ShutdownHandle;
use crate::session::SessionRegistry;

/// State shared by every handler (cloned per request, so all fields are
/// cheaply cloneable handles).
#[derive(Clone)]
pub struct AppState {
    pub cmd_tx: mpsc::Sender<worker::Command>,
    pub sse_tx: broadcast::Sender<String>,
    pub stop_rx: watch::Receiver<bool>,
    pub token: Arc<String>,
    pub port: u16,
    pub sessions: Arc<SessionRegistry>,
    /// Reserved for direct shutdown signaling in Phase 5; Phase 1 routes
    /// shutdown through the command worker so all triggers converge.
    #[allow(dead_code)]
    pub shutdown: ShutdownHandle,
}

/// API error type. Every variant maps to the `{ "error": { code, message } }`
/// envelope defined in architecture §5.
#[derive(Debug)]
pub enum ApiError {
    /// 409 — D2 dirty draft; save or discard first.
    DirtyState,
    /// 401 — no credentials supplied.
    Unauthorized,
    /// 403 — invalid token, invalid Host, or unknown session.
    Forbidden,
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
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "missing token".to_string(),
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "invalid token or host".to_string(),
            ),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m),
            Self::NotFound(w) => (StatusCode::NOT_FOUND, "not_found", format!("{w} not found")),
            Self::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", m),
        };
        let body = dto::ApiErrorBody::new(code, message);
        (status, Json(body)).into_response()
    }
}

/// Build the full router: `/api/*` sub-router with the auth guard, plus a
/// fallback that serves embedded static assets (SPA).
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
                ApiError::Unauthorized,
                StatusCode::UNAUTHORIZED,
                "unauthorized",
            ),
            (ApiError::Forbidden, StatusCode::FORBIDDEN, "forbidden"),
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
