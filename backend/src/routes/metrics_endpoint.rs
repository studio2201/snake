//! `GET /metrics` — Prometheus text-format exporter.
//!
//! Reads the four `AtomicU64` counters + the `version` string from the
//! shared [`Metrics`] handle, refreshes the `active_sessions` and
//! `leaderboard_entries` gauges from the live [`AppState`], and renders
//! the result. No auth, no rate-limit — orchestrators scrape this on a
//! tight schedule.

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::metrics::prometheus_text;
use crate::state::AppState;

/// `GET /metrics` — Prometheus text-format scrape target.
pub async fn metrics_endpoint(State(state): State<AppState>) -> Response {
    let sessions = state.active_session_count().await as u64;
    state.metrics.set_active_sessions(sessions);
    state
        .metrics
        .set_leaderboard_entries(crate::routes::leaderboard::read_leaderboard_count(&state).await);

    let body = prometheus_text(&state.metrics);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::services::rate_limit::RateLimiter;
    use crate::state::AppStateInner;
    use axum::body::to_bytes;
    use axum::http::Request;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::{Mutex, RwLock};
    use tower::ServiceExt;

    #[tokio::test]
    async fn metrics_endpoint_renders_prometheus_text() {
        let tmp = TempDir::new().expect("tempdir");
        // Match the production leaderboard shape (`name`, `score`, `date`)
        // so `read_leaderboard` parses cleanly and reports 1 entry.
        std::fs::write(
            tmp.path().join("leaderboard.json"),
            br#"[{"name":"a","score":1,"date":"2026-07-01T00:00:00Z"}]"#,
        )
        .expect("seed");
        let metrics = Arc::new(crate::metrics::Metrics::new("9.9.9", 0, 1));
        metrics.inc_requests();
        metrics.inc_requests();

        let mut server = shared_backend::server::ServerConfig::from_env(crate::config::APP_BRAND);
        server.base_url = "http://localhost:4501".to_string();
        let cfg = AppConfig {
            server,
            page_history_cookie_age_days: 1,
            node_env: "test".to_string(),
            version: "9.9.9".to_string(),
        };
        let state: AppState = Arc::new(AppStateInner {
            config: cfg,
            data_dir: tmp.path().to_path_buf(),
            leaderboard_file: tmp.path().join("leaderboard.json"),
            web_root: tmp.path().join("frontend"),
            active_sessions: RwLock::new(HashSet::new()),
            rate_limiter: RwLock::new(RateLimiter::new()),
            leaderboard_lock: Arc::new(Mutex::new(())),
            metrics,
        });
        let router = crate::router::build_router(state.clone(), &tmp.path().join("frontend"));

        let resp = router
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("oneshot");
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(ct.starts_with("text/plain"), "got content-type: {ct}");
        let body = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
        let text = std::str::from_utf8(&body).expect("utf8");
        // Two manual increments + one from `metrics_counter_middleware`
        // on the outermost layer = 3 total.
        assert!(
            text.contains("snake_requests_total 3\n"),
            "missing counter; body was: {text}"
        );
        assert!(text.contains("snake_requests_429_total 0\n"));
        assert!(text.contains("snake_leaderboard_entries 1\n"));
        assert!(text.contains("snake_build_info{version=\"9.9.9\"} 1\n"));
    }
}
