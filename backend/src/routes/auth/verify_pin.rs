//! `POST /api/verify-pin` — issue a session cookie after PIN validation.
//!
//! Accepts a PIN between 4 and 64 characters (inclusive) and returns a
//! `Set-Cookie` header carrying a fresh session id. The cookie is then
//! trusted by [`super::is_authenticated`] for subsequent requests.
//!
//! ## PIN length policy
//!
//! The shared [`crate::config::AppConfig::pin`] field enforces
//! the same 4-64 character window, so any value the backend accepts is
//! also a value the operator chose deliberately. The frontend today only
//! presents a 4-digit numeric PIN, but the wider range is supported for
//! future migration without a breaking API change.

use axum::Json;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::CookieJar;
use shared_backend::auth::attempts;
use crate::ip::get_client_ip;
use std::net::SocketAddr;
use std::time::Duration;


use crate::error::AppError;
use crate::services::session::generate_session_id;
use crate::state::AppState;

const MIN_PIN_LEN: usize = 4;
const MAX_PIN_LEN: usize = 64;

/// Request body for [`verify_pin`].
#[derive(serde::Deserialize)]
pub struct VerifyPinPayload {
    /// The PIN the client is asserting. 4-64 characters.
    pub pin: String,
}

/// `POST /api/verify-pin` — validate the PIN and, on success, set the
/// session cookie. Lockout is enforced via the shared
/// `attempts::is_locked_out` machinery.
pub async fn verify_pin(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<VerifyPinPayload>,
) -> Result<Response, AppError> {
    let Some(expected_pin) = state.config.pin.clone() else {
        // No PIN configured — nothing to verify; emit a 200 with no cookie.
        return Ok((StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response());
    };

    let ip_str = get_client_ip(
        &headers,
        addr,
        state.config.trust_proxy,
        &state.config.trusted_proxies,
    );
    let max_attempts = state.config.max_attempts;
    let lockout_dur = Duration::from_secs(state.config.lockout_time_minutes * 60);

    if attempts::is_locked_out(&ip_str, max_attempts, lockout_dur) {
        let remaining = attempts::lockout_remaining_secs(&ip_str, lockout_dur);
        let time_left_min = remaining.div_ceil(60);
        tracing::warn!(
            target: "verify_pin",
            client_ip = %ip_str,
            remaining_secs = remaining,
            "blocked: IP is locked out"
        );
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": format!("Too many attempts. Please try again in {time_left_min} minute(s).")
            })),
        )
            .into_response());
    }

    if payload.pin.len() < MIN_PIN_LEN || payload.pin.len() > MAX_PIN_LEN {
        attempts::record_attempt(&ip_str);
        tracing::info!(
            target: "verify_pin",
            client_ip = %ip_str,
            len = payload.pin.len(),
            "rejected: PIN outside 4-64 char window"
        );
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": "Invalid PIN format"
            })),
        )
            .into_response());
    }

    if !constant_time_eq::constant_time_eq(payload.pin.as_bytes(), expected_pin.as_bytes()) {
        let attempt = attempts::record_attempt(&ip_str);
        let attempts_left = max_attempts.saturating_sub(attempt.count);
        tracing::info!(
            target: "verify_pin",
            client_ip = %ip_str,
            attempt = attempt.count,
            "rejected: PIN mismatch"
        );
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "success": false,
                "error": "Invalid PIN",
                "attemptsLeft": attempts_left,
            })),
        )
            .into_response());
    }

    attempts::reset_attempts(&ip_str);

    let session_id = generate_session_id();
    state.register_session(session_id.clone()).await;

    let secure = crate::cookie_auth::cookie_should_be_secure(&headers, &state.config.base_url);
    let cookie = crate::cookie_auth::build_cookie(&session_id,
        state.config.cookie_max_age_hours,
        secure,
    );
    let jar = jar.add(cookie);

    tracing::info!(
        target: "verify_pin",
        client_ip = %ip_str,
        session_prefix = &session_id[..8.min(session_id.len())],
        "PIN accepted; session issued"
    );

    Ok((jar, Json(serde_json::json!({ "success": true }))).into_response())
}
