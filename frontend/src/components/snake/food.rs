//! Food-related helpers shared by the tick loop, the gold timer, and the
//! restart callback.
//!
//! Centralising [`generate_food`] removes the three duplicated closures
//! that previously existed in `snake_state.rs`. The [`install_gold_timeout`]
//! effect flips the gold flag off and respawns the food after
//! [`GOLD_TIMEOUT_MS`] milliseconds.

use gloo_timers::callback::Timeout;
use yew::prelude::*;

use super::Pos;

/// Snake board width and height in cells.
pub const GRID_SIZE: i32 = 20;

/// Milliseconds the gold food stays on the board before reverting to
/// regular food.
const GOLD_TIMEOUT_MS: u32 = 5000;

/// Maximum random-placement attempts before [`generate_food`] gives up and
/// returns the last sampled cell. Bounds the worst-case runtime of the
/// random placement loop.
const MAX_GENERATE_ATTEMPTS: u32 = 100;

/// Picks a random free cell on the board, avoiding the snake.
///
/// The fallback-after-attempts cap protects against an impossibly crowded
/// board (e.g. near game-over) from looping forever; the snake then
/// collides on the next tick, which is acceptable behaviour.
pub fn generate_food(snake: &[Pos]) -> Pos {
    let mut attempts = 0;
    loop {
        let x = (js_sys::Math::random() * GRID_SIZE as f64) as i32;
        let y = (js_sys::Math::random() * GRID_SIZE as f64) as i32;
        let on_snake = snake.contains(&(x, y));
        if !on_snake || attempts >= MAX_GENERATE_ATTEMPTS {
            return (x, y);
        }
        attempts += 1;
    }
}

/// Restores the previous high score from `localStorage`, or returns `0`
/// when nothing has been persisted.
pub(crate) fn load_high_score() -> u32 {
    if let Some(win) = web_sys::window()
        && let Ok(Some(storage)) = win.local_storage()
        && let Ok(Some(hs_val)) = storage.get_item("snake_high_score")
    {
        return hs_val.parse::<u32>().unwrap_or(0);
    }
    0
}

/// Installs the gold-food timeout effect.
///
/// Whenever `is_gold` becomes `true`, a [`Timeout`] is scheduled for
/// [`GOLD_TIMEOUT_MS`] ms in the future. On fire it clears the gold flag
/// and respawns the food at a free cell via [`generate_food`]. The cleanup
/// closure drops the timeout handle so a fresh timeout can replace it.
pub fn install_gold_timeout(
    is_gold: &UseStateHandle<bool>,
    food: &UseStateHandle<Pos>,
    snake: &UseStateHandle<Vec<Pos>>,
) {
    let is_gold = is_gold.clone();
    let food = food.clone();
    let snake = snake.clone();

    use_effect_with((*is_gold, *food), move |&(gold, _f)| {
        if !gold {
            return Box::new(|| ()) as Box<dyn FnOnce()>;
        }

        let is_gold = is_gold.clone();
        let food = food.clone();
        let snake = snake.clone();
        let timeout = Timeout::new(GOLD_TIMEOUT_MS, move || {
            is_gold.set(false);
            food.set(generate_food(&snake));
        });
        Box::new(move || drop(timeout)) as Box<dyn FnOnce()>
    });
}
