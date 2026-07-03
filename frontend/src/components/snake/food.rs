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
    pick_free_position_with(snake, GRID_SIZE, js_sys::Math::random)
}

/// RNG-injectable variant of [`generate_food`].
///
/// `rng` is called twice per attempt to produce `(x, y)` coordinates within
/// `[0, grid_size)`. Splitting this out from [`generate_food`] lets tests
/// drive the loop deterministically and gives Snake AI / replay features
/// an entry point.
pub fn pick_free_position_with<F: FnMut() -> f64>(
    snake: &[Pos],
    grid_size: i32,
    mut rng: F,
) -> Pos {
    let mut attempts = 0;
    loop {
        let x = (rng() * grid_size as f64) as i32;
        let y = (rng() * grid_size as f64) as i32;
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
#[hook]
pub fn use_gold_timeout(
    is_gold: &UseStateHandle<bool>,
    food: &UseStateHandle<Pos>,
    snake: &UseStateHandle<Vec<Pos>>,
) {
    let is_gold = is_gold.clone();
    let food = food.clone();

    // Store the latest snake coordinates in a ref to avoid stale closure capture
    let snake_ref = use_mut_ref(|| (*snake).clone());
    *snake_ref.borrow_mut() = (*snake).clone();

    let snake_ref_for_timeout = snake_ref.clone();

    use_effect_with((*is_gold, *food), move |&(gold, _f)| {
        if !gold {
            return Box::new(|| ()) as Box<dyn FnOnce()>;
        }

        let is_gold = is_gold.clone();
        let food = food.clone();
        let snake_ref = snake_ref_for_timeout.clone();
        let timeout = Timeout::new(GOLD_TIMEOUT_MS, move || {
            is_gold.set(false);
            let current_snake = snake_ref.borrow();
            food.set(generate_food(&current_snake));
        });
        Box::new(move || drop(timeout)) as Box<dyn FnOnce()>
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn pick_free_position_in_bounds_when_rng_in_unit_range() {
        let snake: Vec<Pos> = Vec::new();
        let pos = pick_free_position_with(&snake, GRID_SIZE, || 0.5);
        assert!(pos.0 >= 0 && pos.0 < GRID_SIZE);
        assert!(pos.1 >= 0 && pos.1 < GRID_SIZE);
        assert_eq!(pos, (10, 10));
    }

    #[wasm_bindgen_test]
    fn pick_free_position_rounds_down_at_upper_edge() {
        let snake: Vec<Pos> = Vec::new();
        // 0.999 * 20 = 19.98 -> 19 as i32.
        let pos = pick_free_position_with(&snake, GRID_SIZE, || 0.999);
        assert_eq!(pos, (19, 19));
    }

    #[wasm_bindgen_test]
    fn pick_free_position_uses_zero_rng_for_origin() {
        let snake: Vec<Pos> = Vec::new();
        let pos = pick_free_position_with(&snake, GRID_SIZE, || 0.0);
        assert_eq!(pos, (0, 0));
    }

    #[wasm_bindgen_test]
    fn pick_free_position_avoids_snake_when_rng_dodges() {
        let snake = vec![(5, 5)];
        let pos = pick_free_position_with(&snake, GRID_SIZE, || 0.0);
        assert_eq!(pos, (0, 0));
    }

    #[wasm_bindgen_test]
    fn pick_free_position_retries_when_snake_hits() {
        // Snake covers (10, 10). The counter-driven rng yields (10, 10)
        // on the first attempt (rejected: on_snake=true), then (0, 0) on
        // the second attempt which is free.
        let snake = vec![(10, 10)];
        let mut calls = 0u32;
        let pos = pick_free_position_with(&snake, GRID_SIZE, || {
            calls += 1;
            // Each attempt calls rng() twice (x, then y). Returns 0.5 for
            // attempt 1 so both coordinates land on (10, 10) -> collision.
            // Returns 0.0 for attempt 2 -> (0, 0) which is free.
            if calls <= 2 { 0.5 } else { 0.0 }
        });
        assert_eq!(pos, (0, 0));
        assert_eq!(calls, 4);
    }

    #[wasm_bindgen_test]
    fn pick_free_position_returns_last_sample_when_grid_full() {
        // Snake fills every cell. The attempts cap forces an early return
        // at the last-sampled (x, y), even though it overlaps the snake.
        let mut snake = Vec::with_capacity((GRID_SIZE * GRID_SIZE) as usize);
        for y in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
                snake.push((x, y));
            }
        }
        let pos = pick_free_position_with(&snake, GRID_SIZE, || 0.5);
        assert!(snake.contains(&pos));
    }

    #[wasm_bindgen_test]
    fn pick_free_position_passes_through_oversized_rng_output() {
        // Documents current behaviour: rng output >= 1.0 is *not* clamped
        // to the grid. `(1.5 * 20.0) as i32 == 30`, so this returns an
        // out-of-bounds cell rather than panicking.
        let snake: Vec<Pos> = Vec::new();
        let pos = pick_free_position_with(&snake, GRID_SIZE, || 1.5);
        assert_eq!(pos, (30, 30));
    }
}
