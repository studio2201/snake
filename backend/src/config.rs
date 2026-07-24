//! Snake application configuration (flat struct).
//!
//! Per-app copy of the prior `crate::config::AppConfig`
//! wrapper. The 16 fields that the shared `ServerConfig` had are
//! inlined here as direct fields of [`AppConfig`], and the env-parsing
//! logic is duplicated per app so each app tunes its own defaults
//! without a one-size-fits-all shared abstraction.

use ipnet::IpNet;
use std::env;
use std::str::FromStr;

const DEFAULT_PORT: u16 = 4502;

/// Snake application configuration.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub site_title: String,
    pub base_url: String,
    pub allowed_origins: String,
    pub pin: Option<String>,
    pub enable_translation: bool,
    pub enable_themes: bool,
    pub enable_print: bool,
    pub show_version: bool,
    pub show_github: bool,
    pub trust_proxy: bool,
    pub trusted_proxies: Vec<IpNet>,
    pub max_attempts: u32,
    pub lockout_time_minutes: u64,
    pub cookie_max_age_hours: i64,
    pub shutdown_drain_seconds: u64,
    pub page_history_cookie_age_days: i64,
    pub node_env: String,
    pub version: String,

}

impl AppConfig {
    /// Canonical brand name surfaced as the default PWA / site title
    /// fallback.
    pub const APP_BRAND: &str = "Snake";

    /// Build a config by reading common env vars.
    pub fn load_from_env(port: u16) -> Self {
        #[cfg(not(test))]
        {
            let _ = dotenvy::from_path("/app/data/.env");
            let _ = dotenvy::dotenv();
        }

        let port = if port != 0 && port != DEFAULT_PORT { port } else { parse_or("PORT", DEFAULT_PORT) };
        let site_title = first_nonempty_env(&[
            "Snake_SITE_TITLE",
            "Snake_TITLE",
            "SITE_TITLE",
        ])
        .unwrap_or_else(|| crate::config::APP_BRAND.to_string());
        let base_url =
            env::var("BASE_URL").unwrap_or_else(|_| format!("http://localhost:{port}"));
        let allowed_origins = env::var("ALLOWED_ORIGINS").unwrap_or_default();
        let pin = first_nonempty_env(&["Snake_PIN", "PIN"]).and_then(|p| {
            let len = p.chars().count();
            if (4..=64).contains(&len) {
                Some(p)
            } else {
                None
            }
        });
        let trust_proxy = parse_bool_env("TRUST_PROXY");
        let trusted_proxies = parse_trusted_proxies("TRUSTED_PROXY_IPS");

        Self {
            port,
            site_title,
            base_url,
            allowed_origins,
            pin,
            enable_translation: parse_bool_env("ENABLE_TRANSLATION"),
            enable_themes: parse_optout_bool_env("ENABLE_THEMES", true),
            enable_print: parse_optout_bool_env("ENABLE_PRINT", true),
            show_version: parse_optout_bool_env("SHOW_VERSION", true),
            show_github: parse_optout_bool_env("SHOW_GITHUB", true),
            trust_proxy,
            trusted_proxies,
            max_attempts: parse_or("MAX_ATTEMPTS", 5u32),
            lockout_time_minutes: parse_or("LOCKOUT_TIME_MINUTES", 15u64),
            cookie_max_age_hours: parse_or("COOKIE_MAX_AGE_HOURS", 24i64),
            shutdown_drain_seconds: parse_or("SHUTDOWN_DRAIN_SECONDS", 5u64),
            page_history_cookie_age_days: 1,
            node_env: "test".to_string(),
            version: "test".to_string(),

        }
    }

    /// Returns `true` if PIN-based authentication is enabled.
    #[must_use]
    pub fn pin_enabled(&self) -> bool {
        self.pin.is_some()
    }

    /// Returns the lockout duration as a `std::time::Duration`.
    #[must_use]
    pub fn lockout_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.lockout_time_minutes * 60)
    }
}

fn parse_or<T>(name: &str, default: T) -> T
where
    T: FromStr,
{
    match env::var(name) {
        Ok(v) => match v.parse() {
            Ok(parsed) => parsed,
            Err(_) => {
                tracing::warn!(
                    target: "config",
                    "{name}={v:?} is not a valid value; using default",
                );
                default
            }
        },
        Err(_) => default,
    }
}

fn parse_bool_env(name: &str) -> bool {
    env::var(name)
        .map(|v| v == "true" || v == "on")
        .unwrap_or(false)
}

fn parse_optout_bool_env(name: &str, default: bool) -> bool {
    env::var(name)
        .map(|v| v != "false" && v != "off")
        .unwrap_or(default)
}

fn first_nonempty_env(names: &[&str]) -> Option<String> {
    for name in names {
        if let Ok(v) = env::var(name)
            && !v.is_empty()
        {
            return Some(v);
        }
    }
    None
}

fn parse_trusted_proxies(name: &str) -> Vec<IpNet> {
    env::var(name)
        .ok()
        .map(|s| {
            s.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .filter_map(|s| IpNet::from_str(s).ok())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_does_not_panic() {
        let cfg = AppConfig::load_from_env(4502);
        assert!(!cfg.site_title.is_empty());
    }

    #[test]
    fn lockout_duration_scales_with_minutes() {
        let cfg = AppConfig::load_from_env(4502);
        let expected =
            std::time::Duration::from_secs(cfg.lockout_time_minutes * 60);
        assert_eq!(cfg.lockout_duration(), expected);
    }
}
