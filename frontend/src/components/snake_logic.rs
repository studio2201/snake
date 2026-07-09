//! Pure game-logic functions for the Snake game.
//!
//! Kept free of Yew hooks and DOM access so it can be exercised from a
//! future WASM test harness and so the [`crate::components::snake::tick`]
//! module stays focused on the timer loop.

pub use crate::components::snake::Pos;

use yew::UseStateHandle;

/// Snapshot of game state consumed by [`apply_tick`].
///
/// Mirrors the `UseStateHandle` set used by [`handle_tick`] but as plain
/// owned values, so the function can be exercised from tests without a
/// renderer.
#[derive(Clone, Debug, PartialEq)]
pub struct PureTickInputs {
    pub snake: Vec<Pos>,
    pub direction: Pos,
    pub next_direction: Pos,
    pub food: Pos,
    pub score: u32,
    pub high_score: u32,
    pub game_over: bool,
    pub is_gold: bool,
    pub grid_size: i32,
}

/// Result of one [`apply_tick`] call.
#[derive(Clone, Debug, PartialEq)]
pub struct PureTickOutputs {
    pub snake: Vec<Pos>,
    pub direction: Pos,
    pub food: Pos,
    pub score: u32,
    pub high_score: u32,
    pub game_over: bool,
    pub is_gold: bool,
}

/// Threshold the gold-food roll has to clear to keep the next food as gold.
pub const GOLD_ROLL_THRESHOLD: f64 = 0.15;

/// Points awarded for eating a regular food cell.
pub const REGULAR_FOOD_POINTS: u32 = 10;

/// Points awarded for eating a gold food cell.
pub const GOLD_FOOD_POINTS: u32 = 30;

/// Advances the snake one step in `next_direction` and returns the new
/// state.
///
/// Mirrors the runtime semantics in [`handle_tick`]:
/// - `direction` is always advanced to `next_direction`.
/// - Wall and self collisions set `game_over` and leave the body, food,
///   and score untouched (matching the runtime early-return).
/// - Eating food grows the snake by one cell, awards points, regenerates
///   `food` at the `new_food` argument, and rolls the next gold flag via
///   `gold_roll < [`GOLD_ROLL_THRESHOLD`].
/// - Otherwise the snake slides forward by dropping its tail.
///
/// `gold_roll` is unused on non-food ticks but always required so the
/// runtime path can call `js_sys::Math::random` exactly once per food
/// consumption.
#[allow(clippy::too_many_arguments)]
pub fn apply_tick(input: PureTickInputs, gold_roll: f64, new_food: Pos) -> PureTickOutputs {
    let direction = input.next_direction;
    let current_snake = input.snake;
    let head = current_snake[0];
    let new_head = (head.0 + direction.0, head.1 + direction.1);

    if new_head.0 < 0
        || new_head.0 >= input.grid_size
        || new_head.1 < 0
        || new_head.1 >= input.grid_size
    {
        return PureTickOutputs {
            snake: current_snake,
            direction,
            food: input.food,
            score: input.score,
            high_score: input.high_score,
            game_over: true,
            is_gold: input.is_gold,
        };
    }
    // Self-collision: when not eating, the tail vacates this tick, so ignore
    // the last body cell (classic snake off-by-one).
    let will_grow = new_head == input.food;
    let body_for_collision: &[(i32, i32)] = if will_grow || current_snake.is_empty() {
        &current_snake
    } else {
        &current_snake[..current_snake.len().saturating_sub(1)]
    };
    if body_for_collision.contains(&new_head) {
        return PureTickOutputs {
            snake: current_snake,
            direction,
            food: input.food,
            score: input.score,
            high_score: input.high_score,
            game_over: true,
            is_gold: input.is_gold,
        };
    }

    let mut next_snake = vec![new_head];
    next_snake.extend_from_slice(&current_snake);

    let (food, score, high_score, is_gold) = if will_grow {
        let points = if input.is_gold {
            GOLD_FOOD_POINTS
        } else {
            REGULAR_FOOD_POINTS
        };
        let new_score = input.score + points;
        let new_high = if new_score > input.high_score {
            new_score
        } else {
            input.high_score
        };
        let next_is_gold = gold_roll < GOLD_ROLL_THRESHOLD;
        (new_food, new_score, new_high, next_is_gold)
    } else {
        next_snake.pop();
        (input.food, input.score, input.high_score, input.is_gold)
    };

    PureTickOutputs {
        snake: next_snake,
        direction,
        food,
        score,
        high_score,
        game_over: input.game_over,
        is_gold,
    }
}

/// Persists a new high score to `localStorage`, if a window is available.
/// Pulled out of [`handle_tick`] so the test path stays free of web APIs.
fn persist_high_score(high_score: u32) {
    if let Some(win) = web_sys::window()
        && let Ok(Some(storage)) = win.local_storage()
    {
        let _ = storage.set_item("snake_high_score", &high_score.to_string());
    }
}

/// Runtime wrapper that delegates to [`apply_tick`] and writes the result
/// back to Yew's [`UseStateHandle`]s.
///
/// Kept as a thin adapter so the bulk of the logic stays testable.
#[allow(clippy::too_many_arguments)]
pub fn handle_tick(
    inputs: PureTickInputs,
    snake: &UseStateHandle<Vec<Pos>>,
    dir: &UseStateHandle<Pos>,
    food: &UseStateHandle<Pos>,
    score: &UseStateHandle<u32>,
    high_score: &UseStateHandle<u32>,
    game_over: &UseStateHandle<bool>,
    is_gold: &UseStateHandle<bool>,
    generate_food: &impl Fn() -> Pos,
) {
    let gold_roll = js_sys::Math::random();
    let next_food = generate_food();
    let high_score_val = inputs.high_score;
    let is_gold_val = inputs.is_gold;
    let result = apply_tick(inputs, gold_roll, next_food);

    dir.set(result.direction);
    if result.game_over {
        game_over.set(true);
        return;
    }
    snake.set(result.snake);
    food.set(result.food);
    score.set(result.score);
    if result.high_score != high_score_val {
        high_score.set(result.high_score);
        persist_high_score(result.high_score);
    }
    if result.is_gold != is_gold_val {
        is_gold.set(result.is_gold);
    }
}
