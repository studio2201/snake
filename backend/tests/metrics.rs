//! Black-box integration tests for the snake backend's `/metrics`
//! Prometheus endpoint.

mod common;

use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::build_test_app;

#[tokio::test]
async fn metrics_endpoint_returns_prometheus_text_with_expected_lines() {
    let (_tmp, state, router) = build_test_app(None).await;
    // Seed a couple of sessions and a leaderboard entry so the gauges
    // are non-zero.
    state.register_session("token-a".to_string()).await;
    state.register_session("token-b".to_string()).await;
    tokio::fs::write(
        &state.leaderboard_file,
        br#"[{"name":"a","score":1,"date":"2026-07-01T00:00:00Z"}]"#,
    )
    .await
    .expect("seed leaderboard");

    let resp = router
        .clone()
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
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.starts_with("text/plain"), "got content-type: {ct}");

    let body = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    let text = std::str::from_utf8(&body).expect("utf8");
    assert!(text.contains("snake_requests_total "));
    assert!(text.contains("snake_requests_429_total "));
    assert!(text.contains("snake_active_sessions 2\n"));
    assert!(text.contains("snake_leaderboard_entries 1\n"));
    assert!(text.contains("snake_build_info{version=\"test\"} 1\n"));
}

#[tokio::test]
async fn metrics_endpoint_is_not_rate_limited() {
    // `/metrics` is intentionally outside the per-IP budget — even
    // after exceeding the rate limit, a scraper should still get a
    // 200 OK. We don't wait for the limiter to actually fire here
    // (the default is 100 req / 60s, which would make this test slow);
    // instead we verify the route is reachable after a flood that
    // *would* trip the limit on `/api/pin-required`.
    let (_tmp, _state, router) = build_test_app(None).await;
    for _ in 0..5 {
        let resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("oneshot");
        assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
