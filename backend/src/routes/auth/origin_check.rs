//! Origin-header CSRF defense middleware (re-export of shared helpers).
use axum::extract::{Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;
use shared_backend::auth::origin_check as oc;

use crate::state::AppState;

pub const ALLOW_NULL_ORIGIN_ENV: &str = oc::ALLOW_NULL_ORIGIN_ENV;
pub fn allow_null_origin_from_env() -> bool {
    oc::allow_null_origin_from_env()
}

// Note: kept without `#[must_use]` because the returned `Result` is `()`-shaped; the `must_use` from `Result` already covers the use.
pub fn assert_origin_allowed(
    req: &Request,
    state: &AppState,
    allow_null_origin: bool,
) -> Result<(), Box<Response>> {
    use axum::http::header;
    let Some(origin) = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    else {
        return Err(Box::new(oc::forbidden_response()));
    };
    if origin.eq_ignore_ascii_case("null") {
        if allow_null_origin {
            return Ok(());
        }
        return Err(Box::new(oc::forbidden_response()));
    }
    let base = state.config.base_url.as_str();
    let is_same_origin = if let Some(host) = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
    {
        oc::strip_scheme(origin) == host
    } else {
        false
    };
    if is_same_origin || oc::origin_matches(origin, base) {
        Ok(())
    } else {
        Err(Box::new(oc::forbidden_response()))
    }
}

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
