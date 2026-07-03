//! Build the authentication `Cookie` value.
//!
//! Single source of truth for `HttpOnly`, `SameSite`, path, and the
//! (clamped) lifetime so every auth-related handler issues identical
//! cookies.

//! Build the authentication `Cookie` value.
//!
//! Single source of truth for `HttpOnly`, `SameSite`, path, and the
//! (clamped) lifetime so every auth-related handler issues identical
//! cookies.

use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration;

use super::COOKIE_NAME;

/// One minute, in seconds — the floor for cookie lifetime when the config
/// value is nonsensical (zero or negative).
const MIN_LIFETIME_SECONDS: u64 = 60;
/// Thirty days — the ceiling for cookie lifetime so a misconfigured
/// `cookie_max_age_hours` can't pin a session forever.
const MAX_LIFETIME_SECONDS: u64 = 30 * 24 * 3600;

/// Construct an authentication cookie carrying `session_id`.
///
/// `max_age_hours` is taken from the shared [`ServerConfig`]; we clamp it
/// to `[1 minute, 30 days]` so a typo in env can't accidentally issue a
/// zero-second or multi-year cookie.
#[must_use]
pub fn build_auth_cookie(session_id: &str, max_age_hours: i64, secure: bool) -> Cookie<'static> {
    let max_age_seconds = clamp_seconds(max_age_hours.saturating_mul(3600));
    Cookie::build((COOKIE_NAME, session_id.to_string()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(Duration::seconds(max_age_seconds as i64))
        .build()
}

/// Build an `expired` auth cookie used to clear the session on logout.
#[must_use]
pub fn build_clear_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(Duration::ZERO)
        .build()
}

fn clamp_seconds(seconds: i64) -> u64 {
    if seconds <= 0 {
        tracing::warn!(
            target: "cookie",
            seconds,
            "cookie lifetime non-positive; clamping to {MIN_LIFETIME_SECONDS}s"
        );
        return MIN_LIFETIME_SECONDS;
    }
    let unsigned = u64::try_from(seconds).unwrap_or(MAX_LIFETIME_SECONDS);
    if unsigned < MIN_LIFETIME_SECONDS {
        tracing::warn!(
            target: "cookie",
            seconds,
            "cookie lifetime too short; clamping to {MIN_LIFETIME_SECONDS}s"
        );
        return MIN_LIFETIME_SECONDS;
    }
    if unsigned > MAX_LIFETIME_SECONDS {
        tracing::warn!(
            target: "cookie",
            seconds,
            "cookie lifetime too long; clamping to {MAX_LIFETIME_SECONDS}s"
        );
        return MAX_LIFETIME_SECONDS;
    }
    unsigned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_cookie_uses_session_id() {
        let c = build_auth_cookie("deadbeef", 24, false);
        assert_eq!(c.name(), COOKIE_NAME);
        assert_eq!(c.value(), "deadbeef");
        assert_eq!(c.path(), Some("/"));
        assert!(c.http_only().unwrap_or(false));
        assert_eq!(c.secure(), Some(false)); // explicit false
    }

    #[test]
    fn auth_cookie_secure_flag_propagates() {
        let c = build_auth_cookie("x", 24, true);
        assert_eq!(c.secure(), Some(true));
    }

    #[test]
    fn clear_cookie_has_zero_max_age() {
        let c = build_clear_cookie(false);
        assert_eq!(c.value(), "");
        assert_eq!(c.max_age(), Some(Duration::ZERO));
    }

    #[test]
    fn clamps_negative_hours_to_one_minute() {
        let c = build_auth_cookie("x", -1, false);
        assert_eq!(
            c.max_age(),
            Some(Duration::seconds(MIN_LIFETIME_SECONDS as i64))
        );
    }

    #[test]
    fn clamps_overly_large_hours_to_thirty_days() {
        let c = build_auth_cookie("x", 24 * 365 * 100, false);
        assert_eq!(
            c.max_age(),
            Some(Duration::seconds(MAX_LIFETIME_SECONDS as i64))
        );
    }
}
