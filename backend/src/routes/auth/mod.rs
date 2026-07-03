//! Authentication routes and middleware.
//!
//! Submodules:
//!
//! - [`cookie`] — build the authentication `Set-Cookie` value
//! - [`logout`] — `POST /api/logout`
//! - [`pin_required`] — `GET /api/pin-required` and `GET /api/config`
//! - [`verify_pin`] — `POST /api/verify-pin`
//! - [`rate_limit`] — request-budget middleware
//! - [`require_pin`] — gate `/api/leaderboard` behind a valid session

pub mod cookie;
pub mod logout;
pub mod pin_required;
pub mod rate_limit;
pub mod require_pin;
pub mod verify_pin;

pub use cookie::build_auth_cookie;
pub use logout::logout;
pub use pin_required::{get_config, pin_required};
pub use rate_limit::rate_limit_middleware;
pub use require_pin::require_pin;
pub use verify_pin::verify_pin;

use crate::state::AppState;
use axum::http::HeaderMap;
use axum_extra::extract::cookie::CookieJar;
use constant_time_eq::constant_time_eq;

/// Name of the cookie that carries the authenticated session token.
pub const COOKIE_NAME: &str = "SNAKE_PIN";

/// `true` when the request presents either:
/// - a session cookie that maps to a currently-registered token, or
/// - an `x-pin` header whose value matches the configured PIN.
///
/// When no PIN is configured (auth disabled), every request is considered
/// authenticated so handlers can run unconditionally.
pub async fn is_authenticated(jar: &CookieJar, state: &AppState, headers: &HeaderMap) -> bool {
    let Some(pin) = state.config.server.pin.as_ref() else {
        return true;
    };

    if let Some(cookie) = jar.get(COOKIE_NAME) {
        return state.session_is_valid(cookie.value()).await;
    }

    if let Some(header_pin) = headers.get("x-pin").and_then(|h| h.to_str().ok()) {
        return constant_time_eq(header_pin.as_bytes(), pin.as_bytes());
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::services::rate_limit::RateLimiter;
    use crate::state::AppStateInner;
    use axum::http::HeaderValue;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    fn build_state(pin: Option<&str>) -> AppState {
        let mut server = shared_backend::server::ServerConfig::from_env(crate::config::APP_BRAND);
        server.pin = pin.map(|s| s.to_string());
        let cfg = AppConfig {
            server,
            page_history_cookie_age_days: 1,
            node_env: "test".to_string(),
            version: "test".to_string(),
        };
        let tmp = TempDir::new().expect("tempdir");
        Arc::new(AppStateInner {
            config: cfg,
            data_dir: tmp.path().to_path_buf(),
            leaderboard_file: tmp.path().join("leaderboard.json"),
            web_root: tmp.path().join("frontend"),
            active_sessions: RwLock::new(HashSet::new()),
            rate_limiter: RwLock::new(RateLimiter::new()),
        })
    }

    #[tokio::test]
    async fn no_pin_configured_implies_authenticated() {
        let state = build_state(None);
        let jar = CookieJar::new();
        let headers = HeaderMap::new();
        assert!(is_authenticated(&jar, &state, &headers).await);
    }

    #[tokio::test]
    async fn valid_session_cookie_authenticates() {
        let state = build_state(Some("12345678"));
        state.register_session("token-a".to_string()).await;
        let jar = CookieJar::new();
        let jar = jar.add(axum_extra::extract::cookie::Cookie::new(
            COOKIE_NAME,
            "token-a",
        ));
        let headers = HeaderMap::new();
        assert!(is_authenticated(&jar, &state, &headers).await);
    }

    #[tokio::test]
    async fn invalid_session_cookie_rejects() {
        let state = build_state(Some("12345678"));
        let jar = CookieJar::new();
        let jar = jar.add(axum_extra::extract::cookie::Cookie::new(
            COOKIE_NAME,
            "token-bogus",
        ));
        let headers = HeaderMap::new();
        assert!(!is_authenticated(&jar, &state, &headers).await);
    }

    #[tokio::test]
    async fn matching_x_pin_header_authenticates() {
        let state = build_state(Some("12345678"));
        let jar = CookieJar::new();
        let mut headers = HeaderMap::new();
        headers.insert("x-pin", HeaderValue::from_static("12345678"));
        assert!(is_authenticated(&jar, &state, &headers).await);
    }

    #[tokio::test]
    async fn wrong_x_pin_header_rejects() {
        let state = build_state(Some("12345678"));
        let jar = CookieJar::new();
        let mut headers = HeaderMap::new();
        headers.insert("x-pin", HeaderValue::from_static("00000000"));
        assert!(!is_authenticated(&jar, &state, &headers).await);
    }

    #[tokio::test]
    async fn no_creds_with_pin_configured_rejects() {
        let state = build_state(Some("12345678"));
        let jar = CookieJar::new();
        let headers = HeaderMap::new();
        assert!(!is_authenticated(&jar, &state, &headers).await);
    }
}
