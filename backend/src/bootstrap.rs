//! Tracing initialisation, config loading, and state construction.
//!
//! `bootstrap` is the single place where side-effects at process startup
//! live. `main.rs` calls exactly one function from this module
//! (`build_runtime`) and then forwards to the router.
//!
//! Tracing-subscriber wiring lives in [`shared_backend::tracing_init`]
//! so this file can stay focused on the config/state flow.

use crate::config::AppConfig;
use crate::metrics::Metrics;
use crate::services::paths::{leaderboard_file, resolve_data_dir, resolve_frontend_dir};
use crate::state::{AppState, AppStateInner};
use shared_backend::tracing_init::{default_log_dir, init_tracing};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

/// Side-effects bundle returned to `main.rs` after startup.
pub struct Runtime {
    /// The fully-built application state.
    pub state: AppState,
    /// Resolved web-root directory, used to wire up `ServeDir`.
    pub web_root: PathBuf,
    /// `PORT` value (default [`DEFAULT_PORT`]).
    pub port: u16,
}

/// Default listening port when `PORT` is unset or unparseable.
pub const DEFAULT_PORT: u16 = 4501;

/// Env-var name for the listening port.
pub const PORT_ENV: &str = "PORT";

/// Resolve the listening port from a raw env value, falling back to
/// [`DEFAULT_PORT`] on missing/invalid input.
#[must_use]
pub fn port_from_env(raw: Option<&str>) -> u16 {
    raw.and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// Resolve the listening port from `PORT` env, falling back to
/// [`DEFAULT_PORT`].
#[must_use]
pub fn resolve_port() -> u16 {
    port_from_env(std::env::var(PORT_ENV).ok().as_deref())
}

/// Load the snake `.env` files, gracefully ignoring missing paths.
///
/// Order: `/app/data/.env` first (container), then the current-directory
/// `.env` for local dev.
pub fn load_dotenv() {
    #[cfg(not(test))]
    {
        let _ = dotenvy::from_path("/app/data/.env");
        let _ = dotenvy::dotenv();
    }
}

/// Construct the application state and spawn the rate-limiter cleanup task.
pub fn build_state(config: AppConfig) -> AppState {
    let data_dir = resolve_data_dir();
    let web_root = resolve_frontend_dir();
    let leaderboard_file = leaderboard_file(&data_dir);

    // Initialise metrics with empty counters; gauges get refreshed on
    // every `/metrics` scrape, so seeding them with zero here is fine.
    let metrics = Arc::new(Metrics::new(config.version.clone(), 0, 0));

    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir: data_dir.clone(),
        leaderboard_file,
        web_root: web_root.clone(),
        active_sessions: RwLock::new(std::collections::HashSet::new()),
        rate_limiter: RwLock::new(crate::services::rate_limit::RateLimiter::new()),
        leaderboard_lock: Arc::new(Mutex::new(())),
        metrics,
    });

    let cleanup_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            cleanup_state.clean_old_rate_limits().await;
        }
    });

    tracing::info!(
        target: "bootstrap",
        data_dir = %state.data_dir.display(),
        web_root = %state.web_root.display(),
        pin_enabled = state.config.pin_enabled(),
        "state initialised"
    );
    // Log the leaderboard file location at startup so operators know
    // where scores persist — important when `SNAKE_DATA_DIR` is set
    // somewhere unexpected. Without this line, finding the file
    // requires code-diving.
    tracing::info!(
        target: "bootstrap",
        leaderboard_file = %state.leaderboard_file.display(),
        "scores persist to this file (atomic temp-file + rename)"
    );

    state
}

/// Run the full startup sequence and return a [`Runtime`] bundle.
pub async fn build_runtime() -> Result<Runtime, crate::error::AppError> {
    init_tracing(default_log_dir().as_deref());

    load_dotenv();

    let port = resolve_port();
    let config = AppConfig::load_from_env(port);
    let state = build_state(config);
    state.ensure_data_dir().await.map_err(|e| {
        tracing::error!(target: "bootstrap", error = %e, "data directory init failed");
        crate::error::AppError::Io(e)
    })?;

    let web_root = state.web_root.clone();
    Ok(Runtime {
        state,
        web_root,
        port,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_from_env_defaults_when_missing() {
        assert_eq!(port_from_env(None), DEFAULT_PORT);
    }

    #[test]
    fn port_from_env_defaults_on_invalid() {
        assert_eq!(port_from_env(Some("not-a-number")), DEFAULT_PORT);
        assert_eq!(port_from_env(Some("-1")), DEFAULT_PORT);
        assert_eq!(port_from_env(Some("999999")), DEFAULT_PORT);
    }

    #[test]
    fn port_from_env_reads_valid_value() {
        assert_eq!(port_from_env(Some("9090")), 9090);
        assert_eq!(port_from_env(Some("0")), 0);
        assert_eq!(port_from_env(Some("65535")), 65535);
    }

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(DEFAULT_PORT, 4501);
        assert_eq!(PORT_ENV, "PORT");
    }
}
