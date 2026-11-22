//! Editor URL constants and the browser-opening helper.
//!
//! The editor is always served from one fixed loopback address. There is no
//! token, session or bootstrap phase: opening the URL is all that is needed.

/// Fixed port the HTTP server binds. There is deliberately no random-port
/// fallback: if this port is occupied, startup fails with a clear error.
pub const EDITOR_PORT: u16 = 4000;

/// The single editor URL used by the tray "Open editor…" action and by the
/// "Copy editor URL" clipboard action.
pub const EDITOR_URL: &str = "http://127.0.0.1:4000/";

/// Build the URL for the tray "Settings…" action from the fixed editor URL.
pub fn settings_url(base: &str) -> String {
    format!("{base}#view=settings")
}

/// Open `url` in the default browser on a short-lived thread so neither the
/// Tokio runtime nor the native message loop is stalled.
pub fn open_editor_url(url: &str) {
    let url = url.to_string();
    if let Err(e) = std::thread::Builder::new()
        .name("zoneflow-browser".into())
        .spawn(move || {
            if let Err(e) = open::that_detached(&url) {
                tracing::warn!(error = %e, "failed to open editor in browser");
            }
        })
    {
        tracing::warn!(error = %e, "failed to spawn browser launch thread");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_url_matches_fixed_port() {
        assert_eq!(EDITOR_PORT, 4000);
        assert_eq!(EDITOR_URL, format!("http://127.0.0.1:{EDITOR_PORT}/"));
    }

    #[test]
    fn tray_urls_use_the_fixed_port() {
        // Both tray actions derive from the same fixed URL constant.
        assert_eq!(
            settings_url(EDITOR_URL),
            "http://127.0.0.1:4000/#view=settings"
        );
        assert!(settings_url(EDITOR_URL).starts_with(EDITOR_URL));
    }
}
