//! Shared helpers for the snake backend integration test suite.
//!
//! Each test file is compiled as a separate binary; `mod common;` pulls
//! these helpers into the test binary at compile time. Individual test
//! files use different subsets, so the helpers are tagged dead-code-ok.

#![allow(dead_code)]

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode};
use serde_json::Value;
use tempfile::TempDir;
use tokio::sync::{Mutex, RwLock};
use tower::ServiceExt;

use backend::config::AppConfig;
use backend::router::build_router;
use backend::services::rate_limit::RateLimiter;
use backend::state::{AppState, AppStateInner};

pub const TEST_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);

pub fn make_state(pin: Option<&str>, data_dir: &Path, web_root: &Path) -> AppState {
    let mut server = shared_backend::server::ServerConfig::from_env("Snake");
    server.pin = pin.map(str::to_string);
    let cfg = AppConfig {
        server,
        page_history_cookie_age_days: 1,
        node_env: "test".to_string(),
        version: "test".to_string(),
    };
    Arc::new(AppStateInner {
        config: cfg,
        data_dir: data_dir.to_path_buf(),
        leaderboard_file: data_dir.join("leaderboard.json"),
        web_root: web_root.to_path_buf(),
        active_sessions: RwLock::new(HashSet::new()),
        rate_limiter: RwLock::new(RateLimiter::new()),
        leaderboard_lock: Arc::new(Mutex::new(())),
        metrics: Arc::new(backend::metrics::Metrics::new("test", 0, 0)),
    })
}

pub async fn build_test_app(pin: Option<&str>) -> (TempDir, AppState, Router) {
    let tmp = TempDir::new().expect("tempdir");
    let data_dir = tmp.path().join("data");
    let web_root = tmp.path().join("web");
    tokio::fs::create_dir_all(&data_dir)
        .await
        .expect("mkdir data");
    tokio::fs::create_dir_all(&web_root)
        .await
        .expect("mkdir web");
    tokio::fs::write(
        web_root.join("index.html"),
        "<!DOCTYPE html><title>{{SITE_TITLE}}</title>",
    )
    .await
    .expect("write index");
    let state: AppState = make_state(pin, &data_dir, &web_root);
    state.ensure_data_dir().await.expect("ensure data");
    (tmp, state.clone(), build_router(state, &web_root))
}

pub fn with_connect_info(req: Request<Body>) -> Request<Body> {
    let (mut parts, body) = req.into_parts();
    parts.extensions.insert(ConnectInfo(TEST_ADDR));
    Request::from_parts(parts, body)
}

pub async fn send(router: &Router, req: Request<Body>) -> (StatusCode, Value, HeaderMap) {
    let resp = router.clone().oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = to_bytes(resp.into_body(), usize::MAX).await.expect("body");
    let value: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, value, headers)
}

pub fn json_post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("req")
}

pub fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("req")
}
