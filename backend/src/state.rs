//! Application state shared across handlers and middleware.
//!
//! `AppState` is a thin alias for `Arc<AppStateInner>` so handlers can
//! accept it as `axum::extract::State<AppState>` directly. The inner
//! struct owns the configuration, the resolved data/web roots, the
//! in-memory active-session set, and the per-IP request-budget limiter.

use std::collections::HashSet;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;
use tokio::sync::{Mutex, RwLock};

pub use crate::config::AppConfig;
use crate::metrics::Metrics;
use crate::services::rate_limit::RateLimiter;

/// Process-global state shared by every request handler.
pub struct AppStateInner {
    /// Snake configuration (port, PIN, CORS, ...).
    pub config: AppConfig,
    /// On-disk data directory (leaderboard, logs).
    pub data_dir: PathBuf,
    /// Convenience path: `<data_dir>/leaderboard.json`.
    pub leaderboard_file: PathBuf,
    /// Resolved web-root directory containing the prebuilt frontend.
    pub web_root: PathBuf,
    /// Set of currently valid session tokens (random hex strings).
    pub active_sessions: RwLock<HashSet<String>>,
    /// Per-IP request-budget limiter.
    pub rate_limiter: RwLock<RateLimiter>,
    /// Serialises concurrent reads/writes of the leaderboard file so two
    /// simultaneous submissions cannot lose data via a read-modify-write
    /// race. The lock is held only across the (small) in-memory operation
    /// and the atomic write to disk; it does NOT span the disk read.
    pub leaderboard_lock: Arc<Mutex<()>>,
    /// Process-wide Prometheus counters. Cheap to clone (bag of atomics),
    /// so middleware/handlers can borrow without `Arc` ceremony.
    pub metrics: Arc<Metrics>,
}

/// Cheap-to-clone handle for [`AppStateInner`].
pub type AppState = Arc<AppStateInner>;

impl AppStateInner {
    /// Ensure the on-disk data directory exists and that an empty
    /// `leaderboard.json` is present.
    pub async fn ensure_data_dir(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.data_dir).await?;
        if fs::metadata(&self.leaderboard_file).await.is_err() {
            tracing::info!(
                target: "state",
                path = %self.leaderboard_file.display(),
                "initialising empty leaderboard file"
            );
            fs::write(&self.leaderboard_file, "[]").await?;
        }
        Ok(())
    }

    /// Record a hit from `ip` and report whether the request is allowed.
    pub async fn check_rate_limit(&self, ip: IpAddr) -> bool {
        self.rate_limiter.write().await.check(ip)
    }

    /// Drop rate-limiter entries that have aged out of the window.
    pub async fn clean_old_rate_limits(&self) {
        self.rate_limiter.write().await.cleanup();
    }

    /// Insert a freshly minted session id.
    pub async fn register_session(&self, id: String) {
        self.active_sessions.write().await.insert(id);
    }

    /// Revoke a session id (e.g. on logout).
    pub async fn revoke_session(&self, id: &str) {
        self.active_sessions.write().await.remove(id);
    }

    /// `true` if `id` is currently a valid session token.
    pub async fn session_is_valid(&self, id: &str) -> bool {
        self.active_sessions.read().await.contains(id)
    }

    /// Number of currently active session tokens (diagnostic only).
    pub async fn active_session_count(&self) -> usize {
        self.active_sessions.read().await.len()
    }

    /// How long since the most recent request hit. Always `Instant::now()`
    /// — present so future diagnostics have a clock to anchor on.
    #[must_use]
    pub fn clock_anchor() -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn build_state(dir: &std::path::Path) -> AppState {
        let leaderboard = crate::services::paths::leaderboard_file(dir);
        Arc::new(AppStateInner {
            config: AppConfig {
                server: shared_backend::server::ServerConfig::from_env(crate::config::APP_BRAND),
                page_history_cookie_age_days: 1,
                node_env: "test".to_string(),
                version: "test".to_string(),
            },
            data_dir: dir.to_path_buf(),
            leaderboard_file: leaderboard,
            web_root: dir.join("frontend"),
            active_sessions: RwLock::new(HashSet::new()),
            rate_limiter: RwLock::new(RateLimiter::new()),
            leaderboard_lock: Arc::new(Mutex::new(())),
            metrics: Arc::new(Metrics::new("test", 0, 0)),
        })
    }

    #[tokio::test]
    async fn ensure_data_dir_creates_leaderboard_when_missing() {
        let tmp = TempDir::new().expect("tempdir");
        let state = build_state(tmp.path());
        state.ensure_data_dir().await.expect("ensure");
        let body = tokio::fs::read_to_string(&state.leaderboard_file)
            .await
            .expect("read");
        assert_eq!(body.trim(), "[]");
    }

    #[tokio::test]
    async fn ensure_data_dir_preserves_existing_leaderboard() {
        let tmp = TempDir::new().expect("tempdir");
        tokio::fs::create_dir_all(tmp.path()).await.expect("mkdir");
        let existing = r#"[{"name":"alice","score":42,"date":"2026-07-01T00:00:00Z"}]"#;
        tokio::fs::write(tmp.path().join("leaderboard.json"), existing)
            .await
            .expect("write");
        let state = build_state(tmp.path());
        state.ensure_data_dir().await.expect("ensure");
        let body = tokio::fs::read_to_string(&state.leaderboard_file)
            .await
            .expect("read");
        assert_eq!(body, existing);
    }

    #[tokio::test]
    async fn rate_limit_blocks_over_budget() {
        let tmp = TempDir::new().expect("tempdir");
        let state = build_state(tmp.path());
        // Tighten the budget to a single request so the test is fast.
        {
            let mut rl = state.rate_limiter.write().await;
            *rl = RateLimiter::with_limits(1, std::time::Duration::from_secs(60));
        }
        let ip = IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
        assert!(state.check_rate_limit(ip).await);
        assert!(!state.check_rate_limit(ip).await);
    }

    #[tokio::test]
    async fn session_lifecycle_register_and_revoke() {
        let tmp = TempDir::new().expect("tempdir");
        let state = build_state(tmp.path());
        assert_eq!(state.active_session_count().await, 0);
        state.register_session("token-a".to_string()).await;
        assert!(state.session_is_valid("token-a").await);
        assert_eq!(state.active_session_count().await, 1);
        state.revoke_session("token-a").await;
        assert!(!state.session_is_valid("token-a").await);
    }
}
