//! HTTP route modules.
//!
//! Each submodule owns one slice of the URL space (assets, pages,
//! leaderboard, ...). All handlers are re-exported flat so `main.rs` and
//! the integration tests can wire them into the router with a single
//! `use routes::*;` line.

pub mod asset_manifest;
pub mod assets;
pub mod auth;
pub mod health;
pub mod leaderboard;
pub mod metrics_endpoint;
pub mod pages;
pub mod pwa_manifest;
pub mod ready;
pub mod redirect;

pub use asset_manifest::serve_asset_manifest;
pub use assets::serve_service_worker;
pub use auth::{
    get_config, logout, origin_check_middleware, pin_required, rate_limit_middleware, require_pin,
    verify_pin,
};
pub use health::health_check;
pub use leaderboard::{get_leaderboard, submit_score};
pub use metrics_endpoint::metrics_endpoint;
pub use pages::{serve_login, serve_root};
pub use pwa_manifest::serve_manifest;
pub use ready::ready_check;
pub use redirect::is_valid_redirect_url;
