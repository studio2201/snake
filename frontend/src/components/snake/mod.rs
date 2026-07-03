//! Snake game state, effects, and callbacks.
//!
//! Split into focused modules so each file stays well under the 250-line
//! ceiling enforced by `.cursorrules`:
//!
//! - [`state`] — the [`SnakeState`](state::SnakeState) aggregate and the
//!   [`use_snake_state`](state::use_snake_state) hook that wires everything
//!   together.
//! - [`food`] — gold food timer and the shared `generate_food` helper.
//! - [`tick`] — the interval loop that drives the game forward.
//! - [`actions`] — the user-facing callbacks (restart, submit score, name
//!   input, resume, dpad, set-next-direction).
//! - [`keys`] — the keyboard event listener.
//!
//! Only the items needed by the view layer ([`Pos`] and
//! [`use_snake_state`](state::use_snake_state)) are re-exported here; the
//! helpers stay behind their full module paths so internal callers can
//! still pin down exactly which subsystem they're touching. The [`Pos`]
//! alias also feeds back through [`crate::components::snake_logic`] so its
//! `Pos` symbol keeps the same identity.

/// A single cell coordinate on the [`crate::components::snake_board`] grid.
pub type Pos = (i32, i32);

pub mod actions;
pub mod food;
pub mod keys;
pub mod state;
pub mod tick;

pub use state::use_snake_state;
