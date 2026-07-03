//! Snake backend library entrypoint.
//!
//! The crate is structured so that `main.rs` is a thin orchestration shell
//! while all real logic lives in [`bootstrap`], [`router`], [`state`],
//! [`config`], [`error`], [`services`], and [`routes`]. Splitting the
//! implementation behind a library root lets integration tests pull the
//! router and state pieces together without spawning a subprocess.
//!
//! ## Layout
//!
//! - [`bootstrap`] — tracing init (delegated to
//!   [`shared_backend::tracing_init`]), env-driven config, state construction
//! - [`router`] — single `build_router(state) -> Router` factory
//! - [`state`] — `AppState` + per-IP request-budget helpers
//! - [`config`] — `AppConfig` (wraps `shared_backend::ServerConfig`)
//! - [`error`] — `AppError` enum implementing `IntoResponse`
//! - [`services`] — leaf helpers (rate limit, session IDs, paths)
//! - [`routes`] — axum handlers and middleware, grouped by area

#![deny(unsafe_code)]

pub mod bootstrap;
pub mod config;
pub mod error;
pub mod metrics;
pub mod router;
pub mod routes;
pub mod services;
pub mod state;

pub use config::AppConfig;
pub use error::AppError;
pub use router::build_router;
pub use state::{AppState, AppStateInner};
