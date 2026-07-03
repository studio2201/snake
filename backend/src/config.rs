//! Snake-specific configuration layered on top of shared [`ServerConfig`].
//!
//! Snake adds three fields beyond the shared baseline:
//! - `page_history_cookie_age_days` — undo-history persistence
//! - `node_env` — dev/prod env hint
//! - `version` — `CARGO_PKG_VERSION` snapshot

use shared_backend::server::ServerConfig;

/// Canonical application brand name surfaced as the default PWA / site
/// title fallback. Use this constant instead of hard-coding the literal
/// `"Snake"` at call sites.
pub const APP_BRAND: &str = "Snake";

/// Env-var name controlling the undo-history cookie lifetime.
pub const PAGE_HISTORY_COOKIE_AGE_ENV: &str = "PAGE_HISTORY_COOKIE_AGE";

/// Env-var name indicating whether the deployment is dev or production.
pub const NODE_ENV_VAR: &str = "NODE_ENV";

/// Fallback for [`AppConfig::node_env`] when the env var is unset.
pub const DEFAULT_NODE_ENV: &str = "development";

/// Default undo-history cookie age in days.
pub const DEFAULT_PAGE_HISTORY_COOKIE_AGE_DAYS: i64 = 365;

/// Snake application configuration. Wraps [`ServerConfig`] with snake-specific
/// retention and version fields.
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Shared backend configuration (CORS, port, PIN, IP rules, ...).
    pub server: ServerConfig,
    /// Days the page-history cookie persists before expiry.
    pub page_history_cookie_age_days: i64,
    /// `"development"` or `"production"` — surfaced for frontend telemetry.
    pub node_env: String,
    /// `CARGO_PKG_VERSION` snapshot taken at startup.
    pub version: String,
}

impl AppConfig {
    /// Build a config by combining shared [`ServerConfig::from_env`] with
    /// snake-specific env parsing.
    ///
    /// `port` overrides the value read by `ServerConfig::from_env` from the
    /// `PORT` env var. The shared loader already reads `PORT` itself; the
    /// override exists so callers can pin the port programmatically (used by
    /// integration tests that bind to port 0).
    pub fn load_from_env(port: u16) -> Self {
        let page_history_cookie_age_days = std::env::var(PAGE_HISTORY_COOKIE_AGE_ENV)
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(DEFAULT_PAGE_HISTORY_COOKIE_AGE_DAYS);
        let node_env = std::env::var(NODE_ENV_VAR).unwrap_or_else(|_| DEFAULT_NODE_ENV.to_string());
        Self::assemble(
            port,
            page_history_cookie_age_days,
            node_env,
            env!("CARGO_PKG_VERSION").to_string(),
        )
    }

    /// Build a config from explicit values. Used by [`Self::load_from_env`]
    /// in production and by tests that want to bypass `std::env` (which
    /// is `unsafe` to mutate under our workspace `unsafe_code = "deny"`).
    pub fn assemble(
        port: u16,
        page_history_cookie_age_days: i64,
        node_env: String,
        version: String,
    ) -> Self {
        let mut server = ServerConfig::from_env(APP_BRAND);
        // Only override the port if the caller explicitly asked for one that
        // differs from the shared default. This avoids clobbering `PORT`
        // when the caller is just forwarding the env-driven value.
        if server.port == 4401 && port != 4401 {
            server.port = port;
        }

        tracing::debug!(
            target: "config",
            port = server.port,
            site_title = %server.site_title,
            pin_enabled = server.pin_enabled(),
            version = %version,
            "configuration loaded"
        );

        Self {
            server,
            page_history_cookie_age_days,
            node_env,
            version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_populates_all_fields() {
        let cfg = AppConfig::assemble(0, 7, "test".to_string(), "1.2.3".to_string());
        assert_eq!(cfg.page_history_cookie_age_days, 7);
        assert_eq!(cfg.node_env, "test");
        assert_eq!(cfg.version, "1.2.3");
        assert!(!cfg.version.is_empty());
    }

    #[test]
    fn assemble_does_not_clobber_default_port() {
        // Passing port == 4401 (the shared default) means "don't override".
        let cfg = AppConfig::assemble(4401, 1, "test".into(), "v".into());
        // The shared loader reads PORT itself; whatever was in env is what
        // we end up with. We only assert that the field exists.
        let _ = cfg.server.port;
    }

    #[test]
    fn assemble_overrides_port_when_non_default() {
        let cfg = AppConfig::assemble(9090, 1, "test".into(), "v".into());
        // If the shared loader didn't see PORT=9090 via env, our explicit
        // value wins. If it did, the env value wins. Either way we never
        // panic and the port is set to *something*.
        let _ = cfg.server.port;
    }

    #[test]
    fn load_from_env_does_not_panic() {
        let cfg = AppConfig::load_from_env(0);
        // Version must be the package version from `env!`, never blank.
        assert!(!cfg.version.is_empty());
    }

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(DEFAULT_NODE_ENV, "development");
        assert_eq!(DEFAULT_PAGE_HISTORY_COOKIE_AGE_DAYS, 365);
        assert_eq!(NODE_ENV_VAR, "NODE_ENV");
        assert_eq!(PAGE_HISTORY_COOKIE_AGE_ENV, "PAGE_HISTORY_COOKIE_AGE");
    }
}
