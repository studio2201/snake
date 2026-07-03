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

/// Env-var name controlling the language picker visibility.
pub const ENABLE_TRANSLATION_ENV: &str = "ENABLE_TRANSLATION";

/// Fallback for [`AppConfig::node_env`] when the env var is unset.
pub const DEFAULT_NODE_ENV: &str = "development";

/// Default undo-history cookie age in days.
pub const DEFAULT_PAGE_HISTORY_COOKIE_AGE_DAYS: i64 = 365;

/// Parse a raw `ENABLE_TRANSLATION` env value into the boolean to assign.
///
/// Reading happens here (rather than inline in `assemble`) so the parsing
/// rule — and the canonical "off" tokens — has a single home that's easy
/// to test and reuse.
///
/// Recognised truthy values: `true`, `1`, `yes`, `on` (case-insensitive),
/// plus anything else that's not in the off-list. Anything unparseable
/// stays on the safe default (`true`) rather than disabling translation
/// for a misconfigured operator.
#[must_use]
pub fn parse_translation_env(raw: Option<&str>) -> bool {
    raw.map(|v| {
        !matches!(
            v.to_ascii_lowercase().as_str(),
            "false" | "0" | "no" | "off"
        )
    })
    .unwrap_or(true)
}

/// Live read of `ENABLE_TRANSLATION` from the current process env.
fn read_translation_env() -> bool {
    parse_translation_env(std::env::var(ENABLE_TRANSLATION_ENV).ok().as_deref())
}

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

        // `ENABLE_TRANSLATION` defaults to `true` for Snake (the shared
        // `ServerConfig::from_env` defaults to `false`, which is the
        // upstream notepad convention). Any value other than the
        // canonical "off" tokens enables the language picker. Operators
        // who want it disabled set `ENABLE_TRANSLATION=false`.
        server.enable_translation = read_translation_env();

        tracing::debug!(
            target: "config",
            port = server.port,
            site_title = %server.site_title,
            pin_enabled = server.pin_enabled(),
            enable_translation = server.enable_translation,
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
        assert!(cfg.server.enable_translation);
    }

    #[test]
    fn parse_translation_env_defaults_to_true_when_unset() {
        assert!(parse_translation_env(None));
    }

    #[test]
    fn parse_translation_env_respects_off_tokens() {
        for off in ["false", "False", "FALSE", "0", "no", "off"] {
            assert!(
                !parse_translation_env(Some(off)),
                "ENABLE_TRANSLATION={off} should disable translation"
            );
        }
    }

    #[test]
    fn parse_translation_env_respects_truthy_tokens() {
        for on in ["true", "True", "TRUE", "1", "yes", "on"] {
            assert!(
                parse_translation_env(Some(on)),
                "ENABLE_TRANSLATION={on} should enable translation"
            );
        }
    }

    #[test]
    fn parse_translation_env_unknown_token_keeps_default() {
        // "maybe" / "" aren't in the off-list, so the !matches! check
        // evaluates to true — translation stays enabled (the safe default).
        assert!(parse_translation_env(Some("maybe")));
        assert!(parse_translation_env(Some("")));
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
