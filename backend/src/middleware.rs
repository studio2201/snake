//! Per-app middleware for Snake: CORS, HSTS, title-injection,
//! security-headers. Each function takes `Arc<crate::config::AppConfig>`
//! instead of the prior shared `Arc<ServerConfig>`.
use axum::extract::{Request, State};
use axum::http::header::{HeaderValue, STRICT_TRANSPORT_SECURITY};
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;

use crate::config::AppConfig;
/// State wrapper for the CORS layer.
#[derive(Clone)]
pub struct CorsState(pub Arc<AppConfig>);

/// Construct a CORS layer from the app's config.
pub fn cors_layer(state: &CorsState) -> tower_http::cors::CorsLayer {
    use tower_http::cors::CorsLayer;
    let origins = state.0.allowed_origins.trim();
    if origins == "*" {
        CorsLayer::very_permissive()
    } else if origins.is_empty() {
        CorsLayer::new().allow_origin(
            tower_http::cors::AllowOrigin::exact("null".parse().unwrap()),
        )
    } else {
        let mut layer = CorsLayer::new();
        for o in origins.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            if let Ok(origin) = o.parse::<axum::http::HeaderValue>() {
                layer = layer.allow_origin(origin);
            }
        }
        layer
    }
}

/// State wrapper for the HSTS layer.
#[derive(Clone)]
pub struct HstsState(pub Arc<AppConfig>);

/// Add HSTS header when the connection is HTTPS.
pub async fn hsts_layer(State(state): State<HstsState>, request: Request, next: Next) -> Response {
    let is_secure = request
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or_else(|| state.0.base_url.starts_with("https"));
    let mut response = next.run(request).await;
    if is_secure {
        response.headers_mut().insert(
            STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    response
}

/// State wrapper for the title-injection middleware.
#[derive(Clone)]
pub struct TitleState(pub Arc<AppConfig>);

/// Replace `{{SITE_TITLE}}` placeholders in HTML responses with the configured `site_title`.
pub async fn title_injection_layer(State(state): State<TitleState>, request: Request, next: Next) -> Response {
    use axum::body::to_bytes;
    use axum::http::header;
    let response = next.run(request).await;
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    if !content_type.starts_with("text/html") {
        return response;
    }
    let (parts, body) = response.into_parts();
    let bytes = match to_bytes(body, 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return axum::response::Response::from_parts(
                parts,
                axum::body::Body::empty(),
            );
        }
    };
    let body_str = String::from_utf8_lossy(&bytes).to_string();
    let replaced = body_str.replace("{{SITE_TITLE}}", &state.0.site_title);
    let new_len = replaced.len();
    let mut new_response = axum::response::Response::new(axum::body::Body::from(replaced));
    *new_response.status_mut() = parts.status;
    *new_response.headers_mut() = parts.headers;
    new_response.headers_mut().remove(header::CONTENT_LENGTH);
    new_response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, HeaderValue::from(new_len));
    new_response
}

/// State wrapper for the security-headers middleware.
#[derive(Clone)]
pub struct SecurityHeadersState(pub Arc<AppConfig>);

/// Add `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`,
/// `Referrer-Policy: strict-origin-when-cross-origin`, and a Yew-friendly
/// `Content-Security-Policy` to every response.
pub async fn security_headers_layer(
    State(_state): State<SecurityHeadersState>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        axum::http::header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        axum::http::header::HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; \
             style-src 'self' 'unsafe-inline'; \
             script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
             img-src 'self' data: blob: https:; \
             connect-src 'self' ws: wss:; \
             font-src 'self'; \
             manifest-src 'self';",
        ),
    );
    response
}

