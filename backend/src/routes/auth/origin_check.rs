//! Origin-header CSRF defense middleware. Pin the `Origin` header on
//! state-changing requests to the configured `base_url`. Applied to
//! `/api/leaderboard` only; non-`POST` methods pass through.

use axum::extract::{Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

/// Env-var enabling the `Origin: null` bypass for `curl` clients.
/// Defaults to `false`.
pub const ALLOW_NULL_ORIGIN_ENV: &str = "ALLOW_NULL_ORIGIN";

/// Truthy values: `"true"`, `"1"`, `"on"` (case-insensitive).
#[must_use]
pub fn allow_null_origin_from_env() -> bool {
    matches!(
        std::env::var(ALLOW_NULL_ORIGIN_ENV).ok().as_deref(),
        Some("true" | "TRUE" | "True" | "1" | "on" | "ON" | "On")
    )
}

/// Inspect the request's `Origin` header. `allow_null_origin` is
/// caller-supplied so unit tests pin the policy without touching
/// process-global environment.
pub fn assert_origin_allowed(
    req: &Request,
    state: &AppState,
    allow_null_origin: bool,
) -> Result<(), Box<Response>> {
    let Some(origin) = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    else {
        tracing::warn!(target: "origin_check", "rejected: missing Origin header");
        return Err(Box::new(forbidden_response()));
    };
    if origin.eq_ignore_ascii_case("null") {
        if allow_null_origin {
            return Ok(());
        }
        tracing::warn!(target: "origin_check", origin = %origin, "rejected: null Origin");
        return Err(Box::new(forbidden_response()));
    }
    let base = state.config.server.base_url.as_str();
    if origin_matches(origin, base) {
        Ok(())
    } else {
        tracing::warn!(target: "origin_check", origin = %origin, base_url = %base, "rejected: cross-origin");
        Err(Box::new(forbidden_response()))
    }
}

/// Match `origin` against `base`. Pass when exact, or when `base` is
/// on localhost/`127.0.0.1` and the origin is the same scheme+host on
/// any port (developer ergonomics).
fn origin_matches(origin: &str, base: &str) -> bool {
    if origin == base {
        return true;
    }
    for prefix in ["http://localhost", "http://127.0.0.1"] {
        if !base.starts_with(prefix) {
            continue;
        }
        if let Some(rest) = origin.strip_prefix(prefix) {
            if rest.is_empty() {
                return true;
            }
            if let Some(port) = rest.strip_prefix(':')
                && !port.is_empty()
                && port.chars().all(|c| c.is_ascii_digit())
            {
                return true;
            }
        }
    }
    false
}

fn forbidden_response() -> Response {
    (
        StatusCode::FORBIDDEN,
        axum::Json(serde_json::json!({ "error": "forbidden" })),
    )
        .into_response()
}

/// Axum middleware. Non-`POST` methods pass through.
pub async fn origin_check_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    if req.method() != Method::POST {
        return next.run(req).await;
    }
    match assert_origin_allowed(&req, &state, allow_null_origin_from_env()) {
        Ok(()) => next.run(req).await,
        Err(resp) => *resp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::services::rate_limit::RateLimiter;
    use crate::state::AppStateInner;
    use axum::body::Body;
    use axum::http::{Method, Request as HttpRequest};
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::{Mutex, RwLock};

    fn build_state(base_url: &str) -> AppState {
        let mut server = shared_backend::server::ServerConfig::from_env(crate::config::APP_BRAND);
        server.base_url = base_url.to_string();
        let cfg = AppConfig {
            server,
            page_history_cookie_age_days: 1,
            node_env: "test".to_string(),
            version: "test".to_string(),
        };
        let tmp = tempfile::TempDir::new().expect("tempdir");
        Arc::new(AppStateInner {
            config: cfg,
            data_dir: tmp.path().to_path_buf(),
            leaderboard_file: tmp.path().join("leaderboard.json"),
            web_root: tmp.path().join("frontend"),
            active_sessions: RwLock::new(HashSet::new()),
            rate_limiter: RwLock::new(RateLimiter::new()),
            leaderboard_lock: Arc::new(Mutex::new(())),
            metrics: Arc::new(crate::metrics::Metrics::new("test", 0, 0)),
        })
    }

    fn post_with_origin(origin: Option<&str>) -> Request {
        let mut b = HttpRequest::builder()
            .method(Method::POST)
            .uri("/api/leaderboard");
        if let Some(o) = origin {
            b = b.header(header::ORIGIN, o);
        }
        b.body(Body::empty()).expect("req")
    }

    fn assert_forbidden(state: &AppState, origin: Option<&str>, allow_null: bool) {
        let resp = assert_origin_allowed(&post_with_origin(origin), state, allow_null)
            .expect_err("should reject");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn missing_origin_rejected() {
        assert_forbidden(&build_state("https://snake.example.com"), None, false);
    }

    #[test]
    fn matching_origin_allowed() {
        let state = build_state("https://snake.example.com");
        assert!(
            assert_origin_allowed(
                &post_with_origin(Some("https://snake.example.com")),
                &state,
                false
            )
            .is_ok()
        );
    }

    #[test]
    fn cross_origin_rejected() {
        assert_forbidden(
            &build_state("https://snake.example.com"),
            Some("https://evil.example.com"),
            false,
        );
    }

    #[test]
    fn null_origin_rejected_by_default() {
        assert_forbidden(
            &build_state("https://snake.example.com"),
            Some("null"),
            false,
        );
    }

    #[test]
    fn null_origin_allowed_when_opt_in() {
        let state = build_state("https://snake.example.com");
        assert!(assert_origin_allowed(&post_with_origin(Some("null")), &state, true).is_ok());
    }

    #[test]
    fn localhost_wildcard_accepts_any_port() {
        let state = build_state("http://localhost:4501");
        for origin in [
            "http://localhost:4501",
            "http://localhost:5173",
            "http://localhost:8080",
        ] {
            assert!(
                assert_origin_allowed(&post_with_origin(Some(origin)), &state, false).is_ok(),
                "expected {origin} to be accepted"
            );
        }
    }

    #[test]
    fn loopback_wildcard_accepts_any_port() {
        let state = build_state("http://127.0.0.1:4501");
        for origin in [
            "http://127.0.0.1:4501",
            "http://127.0.0.1:5173",
            "http://127.0.0.1",
        ] {
            assert!(
                assert_origin_allowed(&post_with_origin(Some(origin)), &state, false).is_ok(),
                "expected {origin} to be accepted"
            );
        }
    }

    #[test]
    fn production_rejects_loopback_origin() {
        let state = build_state("https://snake.example.com");
        let resp = assert_origin_allowed(
            &post_with_origin(Some("http://localhost:4501")),
            &state,
            false,
        )
        .expect_err("reject");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn localhost_does_not_accept_garbage_port() {
        let state = build_state("http://localhost:4501");
        for bad in ["http://localhost:abc", "http://localhost:"] {
            let resp = assert_origin_allowed(&post_with_origin(Some(bad)), &state, false)
                .expect_err("reject");
            assert_eq!(resp.status(), StatusCode::FORBIDDEN, "origin: {bad}");
        }
    }
}
