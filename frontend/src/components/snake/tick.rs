//! Interval loop that advances the snake one cell per tick.
//!
//! The tick speed is inversely proportional to the player's score: every
//! [`SCORE_PER_SPEEDUP`] points shave [`SPEEDUP_STEP_MS`] milliseconds off
//! the interval, clamped at [`MIN_TICK_MS`]. The effect re-runs (and
//! therefore resets the interval) whenever one of its dependencies
//! changes — start/pause/game-over/score.

use gloo_timers::callback::Interval;
use yew::prelude::*;

use super::super::snake_logic::{Pos, handle_tick, PureTickInputs};
use super::food::{GRID_SIZE, generate_food};

/// Tick interval floor. Even at high scores the snake never moves faster
/// than this rate.
const MIN_TICK_MS: u32 = 75;

/// Tick interval when the score is `0`. Acts as the starting speed.
const BASE_TICK_MS: u32 = 170;

/// Number of points required to shave one step off the tick interval.
const SCORE_PER_SPEEDUP: u32 = 20;

/// Milliseconds removed from the tick interval per [`SCORE_PER_SPEEDUP`]
/// points.
const SPEEDUP_STEP_MS: u32 = 15;

/// Bundles the [`UseStateHandle`]s consumed by the tick loop.
///
/// A single struct keeps the helper signature under
/// [`clippy::too_many_arguments`](../../../../clippy.toml) while still
/// allowing each handle to be cloned cheaply (they are reference-counted
/// internally).
#[derive(Clone)]
pub struct TickInputs {
    /// `true` once the player has pressed "PRESS START".
    pub started: UseStateHandle<bool>,
    /// `true` while the game is paused.
    pub paused: UseStateHandle<bool>,
    /// `true` after a collision; freezes the tick loop.
    pub game_over: UseStateHandle<bool>,
    /// Current score, drives tick speed.
    pub score: UseStateHandle<u32>,
    /// Last-applied direction (kept in sync with `next_direction` on tick).
    pub direction: UseStateHandle<Pos>,
    /// Player's most recent direction input; consumed at the next tick.
    pub next_direction: UseStateHandle<Pos>,
    /// Snake body cells, ordered head-first.
    pub snake: UseStateHandle<Vec<Pos>>,
    /// Current food position.
    pub food: UseStateHandle<Pos>,
    /// Persistent high score (mirrored to `localStorage`).
    pub high_score: UseStateHandle<u32>,
    /// `true` while the current food is the gold variant.
    pub is_gold: UseStateHandle<bool>,
}

/// Installs the recurring tick effect described in the module docs.
#[hook]
pub fn use_tick_loop(inputs: TickInputs) {
    let TickInputs {
        started,
        paused,
        game_over,
        score,
        direction,
        next_direction,
        snake,
        food,
        high_score,
        is_gold,
    } = inputs;

    // Create a mutable reference cell to store the latest state values across ticks
    let state_ref = use_mut_ref(|| PureTickInputs {
        snake: (*snake).clone(),
        direction: *direction,
        next_direction: *next_direction,
        food: *food,
        score: *score,
        high_score: *high_score,
        game_over: *game_over,
        is_gold: *is_gold,
        grid_size: GRID_SIZE,
    });

    // Update the ref on every render to ensure the tick loop always sees current state
    {
        let mut m = state_ref.borrow_mut();
        m.snake = (*snake).clone();
        m.direction = *direction;
        m.next_direction = *next_direction;
        m.food = *food;
        m.score = *score;
        m.high_score = *high_score;
        m.game_over = *game_over;
        m.is_gold = *is_gold;
    }

    let is_started = *started;
    let is_paused = *paused;
    let is_game_over = *game_over;
    let score_val = *score;

    let snake_handle = snake.clone();
    let dir_handle = direction.clone();
    let food_handle = food.clone();
    let score_handle = score.clone();
    let high_score_handle = high_score.clone();
    let game_over_handle = game_over.clone();
    let is_gold_handle = is_gold.clone();

    let state_ref_for_tick = state_ref.clone();

    use_effect_with(
        (is_started, is_paused, is_game_over, score_val),
        move |&(st, ps, go, s)| {
            if !st || ps || go {
                return Box::new(|| ()) as Box<dyn FnOnce()>;
            }
            let duration = std::cmp::max(
                MIN_TICK_MS,
                BASE_TICK_MS - (s / SCORE_PER_SPEEDUP) * SPEEDUP_STEP_MS,
            );

            let snake_handle = snake_handle.clone();
            let dir_handle = dir_handle.clone();
            let food_handle = food_handle.clone();
            let score_handle = score_handle.clone();
            let high_score_handle = high_score_handle.clone();
            let game_over_handle = game_over_handle.clone();
            let is_gold_handle = is_gold_handle.clone();
            let state_ref = state_ref_for_tick.clone();

            let interval = Interval::new(duration, move || {
                let pure_inputs = {
                    let m = state_ref.borrow();
                    PureTickInputs {
                        snake: m.snake.clone(),
                        direction: m.direction,
                        next_direction: m.next_direction,
                        food: m.food,
                        score: m.score,
                        high_score: m.high_score,
                        game_over: m.game_over,
                        is_gold: m.is_gold,
                        grid_size: GRID_SIZE,
                    }
                };

                let snake_for_food = pure_inputs.snake.clone();
                handle_tick(
                    pure_inputs,
                    &snake_handle,
                    &dir_handle,
                    &food_handle,
                    &score_handle,
                    &high_score_handle,
                    &game_over_handle,
                    &is_gold_handle,
                    &move || generate_food(&snake_for_food),
                );
            });
            Box::new(move || drop(interval))
        },
    );
}
