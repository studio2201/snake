//! Black-box integration tests for snake-backend edge cases that the
//! existing suites don't cover: service-worker fallback path, PWA
//! manifest overrides, HTML template substitution, invalid cookies,
//! malformed/missing JSON, empty leaderboard, and service-worker
//! content-type.

mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use common::{build_test_app, get, send, with_connect_info};

/// Variant of `send` that returns the raw response body bytes so we can
/// assert on non-JSON content (HTML / JS).
async fn send_raw(
    router: &axum::Router,
    req: Request<Body>,
) -> (StatusCode, Vec<u8>, axum::http::HeaderMap) {
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    (status, body.to_vec(), headers)
}

#[tokio::test]
async fn service_worker_fallback_when_placeholder_missing() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let sw_path = _state.web_root.join("service-worker.js");
    tokio::fs::write(
        &sw_path,
        b"// no version assignment here\nself.addEventListener('fetch', () => {});\n",
    )
    .await
    .expect("write sw");
    let (status, body, _headers) = send_raw(&router, get("/service-worker.js")).await;
    assert_eq!(status, StatusCode::OK);
    let text = std::str::from_utf8(&body).expect("utf8");
    assert!(text.starts_with("// no version assignment here"));
    assert!(
        text.contains(r#"let APP_VERSION = "test";"#),
        "fallback assignment should be appended; body was: {text}"
    );
}

#[tokio::test]
async fn service_worker_route_sets_javascript_content_type() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let sw_path = _state.web_root.join("service-worker.js");
    tokio::fs::write(
        &sw_path,
        br#"self.addEventListener('install', () => {});
let APP_VERSION = "old";
"#,
    )
    .await
    .expect("write sw");
    let (status, _body, headers) = send_raw(&router, get("/service-worker.js")).await;
    assert_eq!(status, StatusCode::OK);
    let ct = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(ct, "application/javascript");
}

#[tokio::test]
async fn pwa_manifest_overrides_site_title() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let (status, body, _headers) = send(&router, get("/Assets/manifest.json")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Snake");
    assert_eq!(body["short_name"], "Snake");
    assert_eq!(body["description"], "A traditional arcade snake game");
}

#[tokio::test]
async fn index_html_replaces_site_title_placeholder() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let (status, body, _headers) = send_raw(&router, get("/")).await;
    assert_eq!(status, StatusCode::OK);
    let text = std::str::from_utf8(&body).expect("utf8");
    assert!(
        text.contains("<title>Snake</title>"),
        "{{SITE_TITLE}} should be replaced; got: {text}"
    );
    assert!(!text.contains("{{SITE_TITLE}}"));
}

#[tokio::test]
async fn leaderboard_post_rejects_unknown_session_cookie() {
    let (_tmp, _state, router) = build_test_app(Some("1234")).await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/leaderboard")
        .header("content-type", "application/json")
        .header("cookie", "SNAKE_PIN=some-unknown-token")
        .header("origin", "http://localhost:4401")
        .body(Body::from(
            json!({ "name": "x", "score": 1, "date": "2026-01-01T00:00:00Z" }).to_string(),
        ))
        .expect("req");
    let (status, body, _) = send(&router, with_connect_info(req)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Unauthorized");
}

#[tokio::test]
async fn leaderboard_post_returns_400_on_malformed_json() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/leaderboard")
        .header("content-type", "application/json")
        .header("origin", "http://localhost:4401")
        .body(Body::from(r#"{"name": "not closed"#))
        .expect("req");
    let (status, _body, _) = send(&router, with_connect_info(req)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn leaderboard_post_returns_422_when_name_missing() {
    let (_tmp, _state, router) = build_test_app(None).await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/leaderboard")
        .header("content-type", "application/json")
        .header("origin", "http://localhost:4401")
        .body(Body::from(
            json!({ "score": 100, "date": "2026-01-01T00:00:00Z" }).to_string(),
        ))
        .expect("req");
    let (status, _body, _) = send(&router, with_connect_info(req)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn leaderboard_get_returns_empty_array_after_file_delete() {
    let (_tmp, state, router) = build_test_app(None).await;
    tokio::fs::remove_file(&state.leaderboard_file)
        .await
        .expect("delete leaderboard file");
    let (status, body, _) = send(&router, with_connect_info(get("/api/leaderboard"))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
    assert_eq!(body.as_array().expect("array").len(), 0);
    let raw = serde_json::to_string(&body).expect("serialise");
    assert_eq!(raw, "[]");
}

#[tokio::test]
async fn request_body_over_limit_returns_413() {
    let (_tmp, _state, router) = build_test_app(None).await;
    // 64 KiB is the configured body limit; send 80 KiB to overflow it.
    let oversize = "x".repeat(80 * 1024);
    let req = Request::builder()
        .method("POST")
        .uri("/api/verify-pin")
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"pin":"{oversize}"}}"#)))
        .expect("req");
    let (status, _body, _) = send(&router, with_connect_info(req)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn leaderboard_concurrent_submissions_do_not_lose_data() {
    use std::collections::HashSet;
    let (_tmp, state, router) = build_test_app(None).await;

    // Fire 10 POSTs concurrently; the leaderboard serialises them via
    // the per-state mutex, so every entry must land in the final read.
    // Each request carries `Origin: http://localhost:4401` so the CSRF
    // middleware on `/api/leaderboard` accepts the submission.
    let mut handles = Vec::new();
    for i in 0..10 {
        let router = router.clone();
        handles.push(tokio::spawn(async move {
            let entry = json!({
                "name": format!("player_{i:02}"),
                "score": i as u32,
                "date": "2026-01-01T00:00:00Z",
            });
            let req = with_connect_info(
                Request::builder()
                    .method("POST")
                    .uri("/api/leaderboard")
                    .header("content-type", "application/json")
                    .header("origin", "http://localhost:4401")
                    .body(Body::from(entry.to_string()))
                    .expect("req"),
            );
            send(&router, req).await.0
        }));
    }
    for h in handles {
        assert_eq!(h.await.expect("join"), StatusCode::OK);
    }

    // Final GET should reflect all 10 unique entries.
    let (status, body, _) = send(&router, with_connect_info(get("/api/leaderboard"))).await;
    assert_eq!(status, StatusCode::OK);
    let names: HashSet<String> = body
        .as_array()
        .expect("array")
        .iter()
        .map(|e| e["name"].as_str().unwrap_or("").to_string())
        .collect();
    assert_eq!(
        names.len(),
        10,
        "expected 10 unique entries, got: {names:?}"
    );

    // Temp file should not linger after the last atomic_write.
    assert!(
        !state
            .leaderboard_file
            .with_file_name(".leaderboard.json.tmp")
            .exists(),
        "atomic-write temp file still present after all writes completed"
    );
}
