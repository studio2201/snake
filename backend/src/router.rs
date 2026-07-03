//! Assemble the final axum [`Router`] from the various route modules.

use axum::Router;
use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use shared_backend::middleware::{HstsState, cors_layer, hsts_layer, security_headers_layer};
use std::path::Path;
use std::sync::Arc;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::routes::{
    get_config, get_leaderboard, health_check, logout, metrics_endpoint, origin_check_middleware,
    pin_required, rate_limit_middleware, ready_check, require_pin, serve_asset_manifest,
    serve_login, serve_manifest, serve_root, serve_service_worker, submit_score, verify_pin,
};
use crate::state::AppState;

/// Hard cap on request bodies for the `/api/*` namespace. The largest
/// legitimate JSON payload is the leaderboard entry
/// (`{name, score, date}`); 64 KiB is generous and still closes the
/// obvious DoS where a client `POST`s multi-MB JSON to /api/* to exhaust
/// memory. Falling outside this limit automatically returns
/// `413 Payload Too Large` via [`RequestBodyLimitLayer`].
const REQUEST_BODY_LIMIT_BYTES: usize = 64 * 1024;

/// Increment `snake_requests_total` for every request that reaches the
/// router. Lives at the outermost layer so even rate-limited and
/// cross-origin-rejected requests show up in the counter; the dedicated
/// `snake_requests_429_total` gauge is incremented separately by
/// [`rate_limit_middleware`] on a `429` response.
async fn metrics_counter_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    state.metrics.inc_requests();
    next.run(req).await
}

/// Build the snake router. `web_root` is the resolved on-disk frontend
/// directory (the `ServeDir` fallback serves anything not handled by an
/// explicit route from there).
pub fn build_router(state: AppState, web_root: &Path) -> Router {
    let server_config = Arc::new(state.config.server.clone());
    let cors = cors_layer(&server_config);

    // Gated sub-router: leaderboard POST/GET requires a valid session,
    // an allow-listed `Origin`, and a fresh rate-limit budget.
    let api_routes = Router::new()
        .route("/leaderboard", get(get_leaderboard).post(submit_score))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            origin_check_middleware,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), require_pin));

    let public_api_routes = Router::new()
        .route("/verify-pin", post(verify_pin))
        .route("/pin-required", get(pin_required))
        .route("/config", get(get_config))
        .route("/logout", post(logout));

    // Merge the gated + public sub-routers, then attach body-limit, rate-limit.
    // Body-limit applied to the merged router so EVERY `/api/*` POST is
    // capped — including PIN verification, which would otherwise be a
    // memory-exhaustion vector.
    let merged_api = api_routes
        .merge(public_api_routes)
        .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT_BYTES))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ));

    Router::new()
        .route("/", get(serve_root))
        .route("/login", get(serve_login))
        .route("/service-worker.js", get(serve_service_worker))
        .route("/asset-manifest.json", get(serve_asset_manifest))
        .route("/Assets/manifest.json", get(serve_manifest))
        .nest("/api", merged_api)
        .route("/health", get(health_check))
        .route("/ready", get(ready_check))
        .route("/metrics", get(metrics_endpoint))
        .fallback_service(
            ServeDir::new(web_root)
                .precompressed_br()
                .precompressed_gzip(),
        )
        .layer(middleware::from_fn_with_state(
            HstsState(server_config.clone()),
            hsts_layer,
        ))
        .layer(middleware::from_fn(security_headers_layer))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics_counter_middleware,
        ))
        .with_state(state)
}
