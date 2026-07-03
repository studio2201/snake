//! Black-box integration tests for the snake backend's Origin-header
//! CSRF middleware on `/api/leaderboard` POSTs.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;

use common::{build_test_app, send, with_connect_info};

fn leaderboard_post(uri: &str, origin: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(o) = origin {
        b = b.header("origin", o);
    }
    b.body(Body::from(
        json!({ "name": "x", "score": 1, "date": "2026-01-01T00:00:00Z" }).to_string(),
    ))
    .expect("req")
}

#[tokio::test]
async fn origin_check_blocks_missing_origin() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let (status, body, _) = send(
        &router,
        with_connect_info(leaderboard_post("/api/leaderboard", None)),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "forbidden");
}

#[tokio::test]
async fn origin_check_blocks_cross_origin() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let (status, body, _) = send(
        &router,
        with_connect_info(leaderboard_post(
            "/api/leaderboard",
            Some("https://evil.example.com"),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "forbidden");
}

#[tokio::test]
async fn origin_check_blocks_null_origin_by_default() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let (status, body, _) = send(
        &router,
        with_connect_info(leaderboard_post("/api/leaderboard", Some("null"))),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "forbidden");
}

#[tokio::test]
async fn origin_check_allows_matching_origin() {
    let (_tmp, _state, router) = build_test_app(None).await;
    // The test state uses the default `base_url` which is
    // `http://localhost:4401` when `BASE_URL` is unset.
    let (status, _body, _) = send(
        &router,
        with_connect_info(leaderboard_post(
            "/api/leaderboard",
            Some("http://localhost:4401"),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn origin_check_does_not_apply_to_get() {
    // GETs skip the CSRF check entirely so a vanilla `fetch()` from
    // any origin can still read the leaderboard.
    let (_tmp, _state, router) = build_test_app(None).await;
    let req = Request::builder()
        .uri("/api/leaderboard")
        .body(Body::empty())
        .expect("req");
    let (status, _body, _) = send(&router, with_connect_info(req)).await;
    assert_eq!(status, StatusCode::OK);
}
