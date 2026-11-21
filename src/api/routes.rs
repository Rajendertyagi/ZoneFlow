//! All §5 HTTP endpoints. Handlers only send commands and await oneshot
//! replies — no blocking engine work runs on the Tokio thread.

use super::dto::{
    AppOptionsDto, AssignDto, MergeDto, SettingsDto, SplitDto, ZoneDto, ZoneGeometryUpdateDto,
};
use super::worker::Command;
use super::{events, middleware, ApiError, AppState};
use crate::api::commands::CommandPayload;
use axum::extract::{Path, Query, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tokio::sync::oneshot;

/// Send one command to the worker and await its reply.
async fn call(state: &AppState, payload: CommandPayload) -> Result<Value, ApiError> {
    let (reply, rx) = oneshot::channel();
    state
        .cmd_tx
        .send(Command { payload, reply })
        .map_err(|e| ApiError::Internal(format!("command worker stopped: {e}")))?;
    rx.await
        .map_err(|e| ApiError::Internal(format!("command reply lost: {e}")))?
}

fn json_response(value: Value) -> Response {
    Json(value).into_response()
}

/* ------------------------------- query types ------------------------------ */

#[derive(Debug, Deserialize)]
struct DiscardQuery {
    #[serde(default)]
    discard_dirty: bool,
}

#[derive(Debug, Deserialize)]
struct AppsQuery {
    #[serde(default = "default_source")]
    source: String,
}

fn default_source() -> String {
    "installed".to_string()
}

#[derive(Debug, Deserialize)]
struct LogsQuery {
    #[serde(default)]
    since: u64,
}

/* -------------------------------- handlers ------------------------------- */

/// `POST /api/session` — exchange the bearer token for an HttpOnly cookie.
async fn session(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
        .trim();
    if bearer != state.token.as_str() {
        return ApiError::Forbidden.into_response();
    }
    let id = state.sessions.create();
    let mut response = (StatusCode::OK, Json(json!({ "ok": true }))).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, middleware::session_cookie(&id));
    response
}

async fn get_state(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::StateGet).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn get_status(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::StatusGet).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn shutdown(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::Shutdown).await {
        Ok(_) => json_response(json!({ "ok": true })),
        Err(e) => e.into_response(),
    }
}

// --------------------------------- profiles --------------------------------

async fn list_profiles(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::ProfilesList).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn create_profile(State(state): State<AppState>, Json(name): Json<Value>) -> Response {
    let name = name
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    match call(&state, CommandPayload::ProfileCreate { name }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn rename_profile(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    let new_name = body
        .get("new_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    match call(&state, CommandPayload::ProfileRename { name, new_name }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn delete_profile(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    match call(&state, CommandPayload::ProfileDelete { name }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn duplicate_profile(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    let new_name = body
        .get("new_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    match call(&state, CommandPayload::ProfileDuplicate { name, new_name }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn load_profile(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<DiscardQuery>,
) -> Response {
    match call(
        &state,
        CommandPayload::ProfileLoad {
            name,
            discard_dirty: q.discard_dirty,
        },
    )
    .await
    {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn select_profile(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<DiscardQuery>,
) -> Response {
    match call(
        &state,
        CommandPayload::ProfileSelect {
            name,
            discard_dirty: q.discard_dirty,
        },
    )
    .await
    {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn apply_profile(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<DiscardQuery>,
) -> Response {
    match call(
        &state,
        CommandPayload::ProfileApply {
            name,
            discard_dirty: q.discard_dirty,
        },
    )
    .await
    {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

// ---------------------------------- zones ----------------------------------

async fn list_zones(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::ZonesList).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn create_zone(State(state): State<AppState>, Json(zone): Json<ZoneDto>) -> Response {
    match call(&state, CommandPayload::ZoneCreate { zone }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn update_zone(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(zone): Json<ZoneDto>,
) -> Response {
    match call(&state, CommandPayload::ZoneUpdate { id, zone }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

/// `PUT /api/zones/batch` — apply several geometry updates as one undo step.
async fn update_zones_batch(
    State(state): State<AppState>,
    Json(updates): Json<Vec<ZoneGeometryUpdateDto>>,
) -> Response {
    match call(&state, CommandPayload::ZonesBatchUpdate { updates }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn delete_zone(State(state): State<AppState>, Path(id): Path<i32>) -> Response {
    match call(&state, CommandPayload::ZoneDelete { id }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn split_zone(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(spec): Json<SplitDto>,
) -> Response {
    match call(&state, CommandPayload::ZoneSplit { id, spec }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn merge_zones(State(state): State<AppState>, Json(spec): Json<MergeDto>) -> Response {
    match call(&state, CommandPayload::ZoneMerge { spec }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn assign_zone(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(spec): Json<AssignDto>,
) -> Response {
    match call(&state, CommandPayload::ZoneAssign { id, spec }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn unassign_zone(State(state): State<AppState>, Path(id): Path<i32>) -> Response {
    match call(&state, CommandPayload::ZoneUnassign { id }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn undo(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::Undo).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn redo(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::Redo).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

// ---------------------------------- apps -----------------------------------

async fn list_apps(State(state): State<AppState>, Query(q): Query<AppsQuery>) -> Response {
    match call(&state, CommandPayload::AppsList { source: q.source }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn browse_apps(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::AppsBrowse).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

// --------------------------------- actions ---------------------------------

async fn save(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::Save).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn apply(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::Apply).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn launch_arrange(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::LaunchArrange).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn preview(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::Preview).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn revert(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::Revert).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn disable(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::Disable).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn enable(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::Enable).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

// --------------------------- settings / hotkeys / logs ---------------------

async fn get_settings(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::SettingsGet).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn set_settings(
    State(state): State<AppState>,
    Json(settings): Json<SettingsDto>,
) -> Response {
    match call(&state, CommandPayload::SettingsSet { settings }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

/// `PUT /api/app-options` — edit the AppConfig-level autostart + close behavior.
async fn set_app_options(
    State(state): State<AppState>,
    Json(options): Json<AppOptionsDto>,
) -> Response {
    match call(&state, CommandPayload::AppOptionsSet { options }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn get_hotkeys(State(state): State<AppState>) -> Response {
    match call(&state, CommandPayload::HotkeysGet).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn set_hotkeys(
    State(state): State<AppState>,
    Json(hotkeys): Json<BTreeMap<String, String>>,
) -> Response {
    match call(&state, CommandPayload::HotkeysSet { hotkeys }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

async fn get_logs(State(state): State<AppState>, Query(q): Query<LogsQuery>) -> Response {
    match call(&state, CommandPayload::LogsGet { since: q.since }).await {
        Ok(v) => json_response(v),
        Err(e) => e.into_response(),
    }
}

/* --------------------------------- router --------------------------------- */

/// Build the full router. The auth guard is a `route_layer` on the `/api`
/// sub-router only; static assets are public via the fallback.
pub fn build(state: AppState) -> Router {
    let api = Router::new()
        .route("/session", post(session))
        .route("/state", get(get_state))
        .route("/events", get(events::events))
        .route("/status", get(get_status))
        .route("/shutdown", post(shutdown))
        .route("/profiles", get(list_profiles).post(create_profile))
        .route(
            "/profiles/{name}",
            put(rename_profile).delete(delete_profile),
        )
        .route("/profiles/{name}/duplicate", post(duplicate_profile))
        .route("/profiles/{name}/load", post(load_profile))
        .route("/profiles/{name}/select", post(select_profile))
        .route("/profiles/{name}/apply", post(apply_profile))
        .route("/zones", get(list_zones).post(create_zone))
        .route("/zones/batch", put(update_zones_batch))
        .route("/zones/{id}", put(update_zone).delete(delete_zone))
        .route("/zones/{id}/split", post(split_zone))
        .route("/zones/merge", post(merge_zones))
        .route("/zones/{id}/assign", post(assign_zone))
        .route("/zones/{id}/assignment", delete(unassign_zone))
        .route("/undo", post(undo))
        .route("/redo", post(redo))
        .route("/apps", get(list_apps))
        .route("/apps/browse", post(browse_apps))
        .route("/save", post(save))
        .route("/apply", post(apply))
        .route("/launch-arrange", post(launch_arrange))
        .route("/preview", post(preview))
        .route("/revert", post(revert))
        .route("/disable", post(disable))
        .route("/enable", post(enable))
        .route("/settings", get(get_settings).put(set_settings))
        .route("/app-options", put(set_app_options))
        .route("/hotkeys", get(get_hotkeys).put(set_hotkeys))
        .route("/logs", get(get_logs))
        .fallback(|| async { ApiError::NotFound("endpoint".to_string()).into_response() })
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_guard,
        ));

    Router::new()
        .nest("/api", api)
        .fallback(crate::web::assets::static_handler)
        .with_state(state)
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::worker::Command as WorkerCommand;
    use crate::native_loop::{HwndSend, NativeQueue, SharedEvent, ShutdownHandle};
    use crate::session::SessionRegistry;
    use axum::body::Body;
    use std::sync::{mpsc, Arc};
    use tokio::sync::{broadcast, watch};
    use tower::ServiceExt;

    struct Harness {
        router: Router,
        #[allow(dead_code)] // kept alive for the duration of the test
        worker: std::thread::JoinHandle<()>,
        token: String,
    }

    fn harness() -> Harness {
        let (cmd_tx, cmd_rx) = mpsc::channel::<WorkerCommand>();
        let (sse_tx, _sse_rx) = broadcast::channel(16);
        let (_stop_tx, stop_rx) = watch::channel(false);
        let queue = Arc::new(NativeQueue::new());
        let shutdown = ShutdownHandle::new(
            queue.clone(),
            SharedEvent::new_test(),
            Arc::new(std::sync::OnceLock::<HwndSend>::new()),
            watch::Sender::new(false),
            None,
        );
        let notifier =
            crate::native_loop::NativeNotifier::new(queue, Arc::new(std::sync::OnceLock::new()));
        let worker = crate::api::worker::spawn_worker(
            cmd_rx,
            crate::config::default_config(),
            sse_tx.clone(),
            notifier,
            shutdown.clone(),
            Arc::new("http://127.0.0.1:1/#token=x".into()),
        );
        let token = "test-token".to_string();
        let state = AppState {
            cmd_tx,
            sse_tx,
            stop_rx,
            token: Arc::new(token.clone()),
            port: 12345,
            sessions: Arc::new(SessionRegistry::new()),
            shutdown,
        };
        Harness {
            router: build(state),
            worker,
            token,
        }
    }

    fn authed(uri: &str, token: &str, method: &str, body: Body) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .header("host", "127.0.0.1:12345")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(body)
            .unwrap()
    }

    async fn body_json(response: Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    }

    #[tokio::test]
    async fn status_and_state_endpoints_work() {
        let h = harness();
        let res = h
            .router
            .clone()
            .oneshot(authed("/api/status", &h.token, "GET", Body::empty()))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        assert_eq!(json["enabled"], true);

        let res = h
            .router
            .clone()
            .oneshot(authed("/api/state", &h.token, "GET", Body::empty()))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        assert_eq!(json["applied"], "Website");
        assert_eq!(json["dirty"], false);
    }

    #[tokio::test]
    async fn profile_crud_endpoints_work() {
        let h = harness();
        let res = h
            .router
            .clone()
            .oneshot(authed(
                "/api/profiles",
                &h.token,
                "POST",
                Body::from(r#"{"name":"Gaming"}"#),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = h
            .router
            .clone()
            .oneshot(authed("/api/profiles", &h.token, "GET", Body::empty()))
            .await
            .unwrap();
        let json = body_json(res).await;
        assert!(json
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "Gaming"));
    }

    #[tokio::test]
    async fn dirty_apply_returns_409() {
        let h = harness();
        // Make the draft dirty via a zone create.
        let res = h
            .router
            .clone()
            .oneshot(authed(
                "/api/zones",
                &h.token,
                "POST",
                Body::from(
                    r#"{"id":0,"label":"A","exe":"a.exe","path":"","x":0,"y":0,"w":50,"h":100}"#,
                ),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = h
            .router
            .clone()
            .oneshot(authed("/api/apply", &h.token, "POST", Body::empty()))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let json = body_json(res).await;
        assert_eq!(json["error"]["code"], "dirty_state");
    }

    #[tokio::test]
    async fn disable_makes_apply_inactive() {
        let h = harness();
        let res = h
            .router
            .clone()
            .oneshot(authed("/api/disable", &h.token, "POST", Body::empty()))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let res = h
            .router
            .clone()
            .oneshot(authed("/api/apply", &h.token, "POST", Body::empty()))
            .await
            .unwrap();
        let json = body_json(res).await;
        assert_eq!(json["inactive"], true);
    }

    #[tokio::test]
    async fn session_sets_cookie() {
        let h = harness();
        let res = h
            .router
            .clone()
            .oneshot(authed("/api/session", &h.token, "POST", Body::empty()))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let cookie = res.headers()[SET_COOKIE].to_str().unwrap();
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
    }

    #[tokio::test]
    async fn app_options_endpoint_updates_close_action() {
        let h = harness();
        let res = h
            .router
            .clone()
            .oneshot(authed(
                "/api/app-options",
                &h.token,
                "PUT",
                Body::from(r#"{"auto_start":false,"close_action":"exit","gap":12}"#),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        assert_eq!(json["ok"], true);

        let res = h
            .router
            .clone()
            .oneshot(authed("/api/state", &h.token, "GET", Body::empty()))
            .await
            .unwrap();
        let json = body_json(res).await;
        assert_eq!(json["draft"]["close_action"], "exit");
    }

    #[tokio::test]
    async fn static_merge_route_wins_over_dynamic_zone_id() {
        let h = harness();
        // POST /zones/merge must hit the static merge route, not /zones/{id}
        // (which has no POST and would answer 405). A missing body yields 400
        // from the merge handler, proving the static route was selected.
        let res = h
            .router
            .clone()
            .oneshot(authed(
                "/api/zones/merge",
                &h.token,
                "POST",
                Body::from("{}"),
            ))
            .await
            .unwrap();
        assert_ne!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_ne!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn batch_zone_update_route_is_one_undo_step() {
        let h = harness();
        // Seed two adjacent zones in the active profile.
        for (label, x) in [("A", 0.0), ("B", 50.0)] {
            let res = h
                .router
                .clone()
                .oneshot(authed(
                    "/api/zones",
                    &h.token,
                    "POST",
                    Body::from(format!(
                        r#"{{"id":0,"label":"{label}","exe":"z.exe","path":"","x":{x},"y":0,"w":50,"h":100}}"#
                    )),
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }

        // One PUT updates both zones. The static /zones/batch route must win
        // over /zones/{id} (which would try to parse "batch" as an i32).
        let res = h
            .router
            .clone()
            .oneshot(authed(
                "/api/zones/batch",
                &h.token,
                "PUT",
                Body::from(
                    r#"[{"id":1,"x":0,"y":0,"w":30,"h":100},{"id":2,"x":30,"y":0,"w":70,"h":100}]"#,
                ),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        assert_eq!(json.as_array().unwrap().len(), 2);

        // A single undo restores both zones to their pre-drag geometry.
        let res = h
            .router
            .clone()
            .oneshot(authed("/api/undo", &h.token, "POST", Body::empty()))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = h
            .router
            .clone()
            .oneshot(authed("/api/state", &h.token, "GET", Body::empty()))
            .await
            .unwrap();
        let json = body_json(res).await;
        let zones = json["zones"].as_array().unwrap();
        let z1 = zones.iter().find(|z| z["id"] == 1).unwrap();
        let z2 = zones.iter().find(|z| z["id"] == 2).unwrap();
        assert_eq!(z1["w"], 50.0);
        assert_eq!(z2["x"], 50.0);
    }

    #[tokio::test]
    async fn unknown_api_path_is_404() {
        let h = harness();
        let res = h
            .router
            .clone()
            .oneshot(authed(
                "/api/does-not-exist",
                &h.token,
                "GET",
                Body::empty(),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn static_fallback_is_public() {
        let h = harness();
        let req = axum::http::Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let res = h.router.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
