//! Per-launch session token, browser session registry and 60-second reaper.
//!
//! The 256-bit token is generated at startup and placed in the URL fragment.
//! `POST /api/session` exchanges it for an `HttpOnly; SameSite=Strict` cookie;
//! subsequent fetch/EventSource requests authenticate with that cookie.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Sessions expire 60 seconds after their last use (architecture §8).
pub const SESSION_TTL: Duration = Duration::from_secs(60);

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// URL-safe base64 without padding (base64url). Implemented locally so no
/// extra crate is needed for a 32-byte token.
pub fn base64url(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() * 4).div_ceil(3));
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | data[i + 2] as u32;
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        out.push(B64[((n >> 6) & 63) as usize] as char);
        out.push(B64[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = (data[i] as u32) << 16;
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
    } else if rem == 2 {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        out.push(B64[((n >> 6) & 63) as usize] as char);
    }
    out
}

/// Generate a 256-bit (32-byte) per-launch token as a 43-character base64url
/// string. Fails loudly if the OS RNG is unavailable — never a weak fallback.
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    if let Err(e) = getrandom::fill(&mut bytes) {
        panic!("OS random number generator unavailable for session token: {e}");
    }
    base64url(&bytes)
}

/// Registry of browser sessions created by `POST /api/session`.
pub struct SessionRegistry {
    sessions: Mutex<HashMap<String, Instant>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Create a new session id and record its creation time.
    pub fn create(&self) -> String {
        let mut bytes = [0u8; 16];
        if let Err(e) = getrandom::fill(&mut bytes) {
            panic!("OS random number generator unavailable for session id: {e}");
        }
        let id = base64url(&bytes);
        match self.sessions.lock() {
            Ok(mut map) => {
                map.insert(id.clone(), Instant::now());
            }
            Err(e) => tracing::warn!(error = %e, "session registry lock poisoned"),
        }
        id
    }

    /// True when the id exists and has not expired. Access also refreshes the
    /// last-seen time so an active editor session never expires mid-use.
    pub fn validate(&self, id: &str) -> bool {
        match self.sessions.lock() {
            Ok(mut map) => match map.get_mut(id) {
                Some(last) if last.elapsed() < SESSION_TTL => {
                    *last = Instant::now();
                    true
                }
                Some(_) => {
                    map.remove(id);
                    false
                }
                None => false,
            },
            Err(e) => {
                tracing::warn!(error = %e, "session registry lock poisoned");
                false
            }
        }
    }

    /// Remove expired sessions. Called by the 60-second reaper task.
    pub fn reap(&self) {
        match self.sessions.lock() {
            Ok(mut map) => map.retain(|_, last| last.elapsed() < SESSION_TTL),
            Err(e) => tracing::warn!(error = %e, "session registry lock poisoned during reap"),
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.sessions.lock().map(|m| m.len()).unwrap_or(0)
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn the periodic reaper on the Tokio runtime. Uses `send_replace`-style
/// idempotence: reaping an empty registry is harmless.
pub fn spawn_reaper(registry: std::sync::Arc<SessionRegistry>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SESSION_TTL);
        ticker.tick().await; // first tick completes immediately
        loop {
            ticker.tick().await;
            registry.reap();
        }
    });
}

/// Open the editor in the default browser. Runs on a short-lived thread so
/// neither the Tokio runtime nor the native message loop is stalled.
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

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_known_vectors() {
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(b"foob"), "Zm9vYg");
        assert_eq!(base64url(b"fooba"), "Zm9vYmE");
        assert_eq!(base64url(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64url_uses_url_safe_alphabet() {
        // 0xFF 0xFF 0xFF would produce '+', '/' in standard base64.
        let s = base64url(&[0xFF, 0xFF, 0xFF]);
        assert_eq!(s, "____");
        assert!(!s.contains('+') && !s.contains('/') && !s.contains('='));
    }

    #[test]
    fn token_is_256_bits_encoded() {
        let token = generate_token();
        assert_eq!(token.len(), 43);
        assert!(token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        assert_ne!(token, generate_token());
    }

    #[test]
    fn session_create_validate_and_reap() {
        let registry = SessionRegistry::new();
        let id = registry.create();
        assert_eq!(registry.len(), 1);
        assert!(registry.validate(&id));
        assert!(!registry.validate("unknown"));
        registry.reap();
        assert_eq!(registry.len(), 1); // still fresh
    }

    #[test]
    fn expired_session_is_reaped() {
        let registry = SessionRegistry::new();
        let id = registry.create();
        {
            let mut map = registry.sessions.lock().unwrap();
            let last = map.get_mut(&id).unwrap();
            *last = Instant::now() - SESSION_TTL - Duration::from_secs(1);
        }
        assert!(!registry.validate(&id));
        registry.reap();
        assert_eq!(registry.len(), 0);
    }
}
