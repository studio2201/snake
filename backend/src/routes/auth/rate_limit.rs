//! Per-IP request-budget middleware.

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use shared_backend::server::get_client_ip;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::state::AppState;

/// Apply the per-IP request budget. Runs *after* [`super::require_pin`], so
/// authenticated clients can still be throttled for misbehaviour.
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let addr = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0);

    let ip = client_ip_from_request(&req, addr, &state);
    let ip_key: IpAddr = ip.parse().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

    if !state.check_rate_limit(ip_key).await {
        tracing::warn!(
            target: "rate_limit",
            client_ip = %ip_key,
            "request budget exhausted"
        );
        state.metrics.inc_rate_limited();
        let body = serde_json::json!({
            "error": "Too many requests. Please slow down."
        });
        let mut response = axum::response::Json(body).into_response();
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        return response;
    }

    next.run(req).await
}

fn client_ip_from_request(req: &Request, addr: Option<SocketAddr>, state: &AppState) -> String {
    let fallback = addr.unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0)));
    get_client_ip(
        req.headers(),
        fallback,
        state.config.server.trust_proxy,
        &state.config.server.trusted_proxies,
    )
}
