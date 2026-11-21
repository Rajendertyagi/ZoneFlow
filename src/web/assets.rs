//! Embedded static assets served by Axum (no separate frontend server).
//!
//! `rust-embed` embeds `dist/` at compile time in release builds and reads it
//! from disk in debug builds. `Metadata` has no `mimetype()` in 8.14, so the
//! Content-Type is derived from the path suffix.

use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "dist/"]
struct Assets;

/// Derive a Content-Type from a path suffix. Unknown extensions get
/// `application/octet-stream`; extensionless SPA routes get HTML via the
/// fallback below.
pub fn content_type(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" | "cjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn embedded_response(path: &str, ct: &'static str) -> Option<Response> {
    let file = Assets::get(path)?;
    Some(([(CONTENT_TYPE, ct)], Body::from(file.data.into_owned())).into_response())
}

/// Static handler with SPA fallback: extensionless paths that are not assets
/// serve `index.html` so client-side routes work on reload. Missing assets
/// with an extension return 404.
pub async fn static_handler(uri: Uri) -> Response {
    let raw = uri.path().trim_start_matches('/');
    let path = if raw.is_empty() { "index.html" } else { raw };

    if let Some(response) = embedded_response(path, content_type(path)) {
        return response;
    }

    let has_extension = std::path::Path::new(path)
        .extension()
        .is_some_and(|e| !e.is_empty());
    if !has_extension {
        if let Some(response) = embedded_response("index.html", "text/html; charset=utf-8") {
            return response;
        }
    }

    tracing::debug!(path = %path, "static asset not found");
    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn content_types_by_extension() {
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(
            content_type("assets/app.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type("assets/app.css"), "text/css; charset=utf-8");
        assert_eq!(content_type("data.json"), "application/json; charset=utf-8");
        assert_eq!(content_type("icon.svg"), "image/svg+xml");
        assert_eq!(content_type("font.woff2"), "font/woff2");
        assert_eq!(content_type("module.wasm"), "application/wasm");
        assert_eq!(content_type("unknown.xyz"), "application/octet-stream");
    }

    #[tokio::test]
    async fn root_serves_index_html() {
        let res = static_handler(Uri::from_static("/")).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[CONTENT_TYPE], "text/html; charset=utf-8");
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        assert!(String::from_utf8_lossy(&body).contains("ZoneFlow"));
    }

    #[tokio::test]
    async fn extensionless_path_falls_back_to_index() {
        let res = static_handler(Uri::from_static("/editor/zones")).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[CONTENT_TYPE], "text/html; charset=utf-8");
    }

    #[tokio::test]
    async fn missing_asset_is_404() {
        let res = static_handler(Uri::from_static("/assets/does-not-exist.js")).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
