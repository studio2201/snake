//! Black-box integration tests for the snake backend's `/ready` probe.

mod common;

use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::{build_test_app, get};

#[tokio::test]
async fn ready_returns_200_when_data_dir_writable_and_leaderboard_present() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let resp = router
        .clone()
        .oneshot(get("/ready"))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    let text = std::str::from_utf8(&body).expect("utf8");
    let parsed: serde_json::Value = serde_json::from_str(text).expect("json");
    assert_eq!(parsed["ready"], serde_json::json!(true));
}

#[tokio::test]
async fn ready_returns_503_when_data_dir_unwritable() {
    // Build a state whose `data_dir` is a child of an existing *file*,
    // which guarantees every write attempt fails. We can't mutate
    // `AppState` after construction (`AppStateInner` is not `Clone`),
    // so we hand-build a router and state via the public surface
    // instead of going through `build_test_app`.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let blocker = tmp.path().join("blocker");
    std::fs::write(&blocker, b"x").expect("blocker");

    let data_dir = blocker.join("subdir");
    let web_root = tmp.path().join("web");
    std::fs::create_dir_all(&web_root).expect("web");
    std::fs::write(web_root.join("index.html"), b"<html></html>").expect("index");

    let mut server = shared_backend::server::ServerConfig::from_env("Snake");
    server.base_url = "http://localhost:4401".to_string();
    let cfg = backend::config::AppConfig {
        server,
        page_history_cookie_age_days: 1,
        node_env: "test".to_string(),
        version: "test".to_string(),
    };
    let leaderboard = data_dir.join("leaderboard.json");
    let state: backend::state::AppState = std::sync::Arc::new(backend::state::AppStateInner {
        config: cfg,
        data_dir: data_dir.clone(),
        leaderboard_file: leaderboard,
        web_root: web_root.clone(),
        active_sessions: tokio::sync::RwLock::new(std::collections::HashSet::new()),
        rate_limiter: tokio::sync::RwLock::new(backend::services::rate_limit::RateLimiter::new()),
        leaderboard_lock: std::sync::Arc::new(tokio::sync::Mutex::new(())),
        metrics: std::sync::Arc::new(backend::metrics::Metrics::new("test", 0, 0)),
    });
    let router = backend::router::build_router(state, &web_root);

    let resp = router
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    let text = std::str::from_utf8(&body).expect("utf8");
    let parsed: serde_json::Value = serde_json::from_str(text).expect("json");
    assert_eq!(parsed["ready"], serde_json::json!(false));
    assert!(
        parsed["reason"]
            .as_str()
            .unwrap_or("")
            .starts_with("data_dir not writable"),
        "unexpected reason: {text}"
    );
}
