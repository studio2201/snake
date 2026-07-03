//! `/ready` readiness probe.
//!
//! Distinct from `/health` (liveness). `/health` is a cheap process-up
//! heartbeat; `/ready` checks the on-disk prerequisites the service
//! actually needs to serve traffic:
//!
//! 1. `data_dir` is writable (write a `.ready.tmp` probe file then
//!    remove it), and
//! 2. the leaderboard file exists or can be created.
//!
//! Returns `200 {"ready":true}` on success, `503 {"ready":false,"reason":...}`
//! on failure. Never authenticated, never rate-limited, never logged at
//! info level — the endpoint is called frequently by orchestrators and
//! log noise would dwarf the signal.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tokio::fs;

use crate::state::AppState;

/// Name of the probe file written and immediately removed to confirm
/// `data_dir` is writable. The leading `.` keeps it out of any default
/// file listing and signals "scratch" to humans who see it linger.
const READY_PROBE_FILENAME: &str = ".ready.tmp";

/// `GET /ready` — return 200 if the service can serve traffic, 503 otherwise.
pub async fn ready_check(State(state): State<AppState>) -> Response {
    match check_ready(&state).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "ready": true }))).into_response(),
        Err(reason) => {
            tracing::warn!(
                target: "ready",
                reason = %reason,
                "readiness check failed"
            );
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "ready": false, "reason": reason })),
            )
                .into_response()
        }
    }
}

/// Run the readiness checks. Returns the human-readable failure reason
/// on the first check that fails. Exposed for tests.
pub async fn check_ready(state: &AppState) -> Result<(), String> {
    let probe_path = state.data_dir.join(READY_PROBE_FILENAME);
    if let Err(e) = fs::write(&probe_path, b"ok").await {
        return Err(format!("data_dir not writable: {e}"));
    }
    if let Err(e) = fs::remove_file(&probe_path).await {
        return Err(format!("data_dir cleanup failed: {e}"));
    }

    match fs::metadata(&state.leaderboard_file).await {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Try to create it; this catches the case where the parent
            // dir exists but the file was deleted, or perms on the
            // directory prevent new files.
            if let Err(create_err) = fs::write(&state.leaderboard_file, b"[]").await {
                Err(format!(
                    "leaderboard file missing and cannot be created: {create_err}"
                ))
            } else {
                Ok(())
            }
        }
        Err(e) => Err(format!("leaderboard file unreadable: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::services::rate_limit::RateLimiter;
    use crate::state::AppStateInner;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::{Mutex, RwLock};

    fn build_state(tmp: &TempDir, leaderboard_exists: bool) -> AppState {
        let mut server = shared_backend::server::ServerConfig::from_env(crate::config::APP_BRAND);
        server.base_url = "http://localhost:4501".to_string();
        let cfg = AppConfig {
            server,
            page_history_cookie_age_days: 1,
            node_env: "test".to_string(),
            version: "test".to_string(),
        };
        let leaderboard = tmp.path().join("leaderboard.json");
        if leaderboard_exists {
            std::fs::write(&leaderboard, b"[]").expect("seed leaderboard");
        }
        Arc::new(AppStateInner {
            config: cfg,
            data_dir: tmp.path().to_path_buf(),
            leaderboard_file: leaderboard,
            web_root: tmp.path().join("frontend"),
            active_sessions: RwLock::new(HashSet::new()),
            rate_limiter: RwLock::new(RateLimiter::new()),
            leaderboard_lock: Arc::new(Mutex::new(())),
            metrics: Arc::new(crate::metrics::Metrics::new("test", 0, 0)),
        })
    }

    #[tokio::test]
    async fn ready_passes_when_data_dir_writable_and_leaderboard_present() {
        let tmp = TempDir::new().expect("tempdir");
        let state = build_state(&tmp, true);
        assert!(check_ready(&state).await.is_ok());
    }

    #[tokio::test]
    async fn ready_passes_when_leaderboard_missing_but_creatable() {
        let tmp = TempDir::new().expect("tempdir");
        let state = build_state(&tmp, false);
        assert!(check_ready(&state).await.is_ok());
        // The handler should have written an empty leaderboard file.
        assert!(state.leaderboard_file.exists());
    }

    #[tokio::test]
    async fn ready_fails_when_data_dir_not_writable() {
        let tmp = TempDir::new().expect("tempdir");
        // Build a state whose data_dir is a child of an existing file,
        // which guarantees the write will fail. `AppStateInner` is not
        // `Clone`, so we reach in via `Arc::get_mut` (safe because no
        // other handle exists yet at this point).
        let blocking_file = tmp.path().join("blocker");
        std::fs::write(&blocking_file, b"x").expect("blocker");
        let mut server = shared_backend::server::ServerConfig::from_env(crate::config::APP_BRAND);
        server.base_url = "http://localhost:4501".to_string();
        let cfg = AppConfig {
            server,
            page_history_cookie_age_days: 1,
            node_env: "test".to_string(),
            version: "test".to_string(),
        };
        let leaderboard = tmp.path().join("leaderboard.json");
        std::fs::write(&leaderboard, b"[]").expect("seed leaderboard");
        let state: AppState = Arc::new(AppStateInner {
            config: cfg,
            data_dir: blocking_file.join("subdir"),
            leaderboard_file: leaderboard,
            web_root: tmp.path().join("frontend"),
            active_sessions: RwLock::new(HashSet::new()),
            rate_limiter: RwLock::new(RateLimiter::new()),
            leaderboard_lock: Arc::new(Mutex::new(())),
            metrics: Arc::new(crate::metrics::Metrics::new("test", 0, 0)),
        });
        let err = check_ready(&state).await.expect_err("should fail");
        assert!(err.starts_with("data_dir not writable"), "got: {err}");
    }
}
