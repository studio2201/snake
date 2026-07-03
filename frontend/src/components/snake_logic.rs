//! Pure game-logic functions for the Snake game.
//!
//! Kept free of Yew hooks and DOM access so it can be exercised from a
//! future WASM test harness and so the [`crate::components::snake::tick`]
//! module stays focused on the timer loop.

pub use crate::components::snake::Pos;

use yew::UseStateHandle;

/// Advances the snake one step in `next_dir`.
///
/// Behaviour:
/// - Moves the head by `next_dir`; updates `direction`.
/// - Sets `game_over` if the head leaves the grid or collides with the
///   body. The early-return form here avoids accidental fall-through into
///   the score update below.
/// - If the head lands on `food`, grows the snake by one, awards points
///   (30 for gold, 10 otherwise), updates `high_score` (and persists it),
///   rolls a new gold/normal food, and respawns the food at a free cell
///   via `generate_food`.
/// - Otherwise just slides the snake forward by dropping the tail cell.
#[allow(clippy::too_many_arguments)]
pub fn handle_tick(
    snake: &UseStateHandle<Vec<Pos>>,
    dir: &UseStateHandle<Pos>,
    next_dir: &UseStateHandle<Pos>,
    food: &UseStateHandle<Pos>,
    score: &UseStateHandle<u32>,
    high_score: &UseStateHandle<u32>,
    game_over: &UseStateHandle<bool>,
    is_gold: &UseStateHandle<bool>,
    grid_size: i32,
    generate_food: &impl Fn() -> Pos,
) {
    let current_dir = **next_dir;
    dir.set(current_dir);
    let current_snake = (**snake).clone();
    let head = current_snake[0];
    let new_head = (head.0 + current_dir.0, head.1 + current_dir.1);

    if new_head.0 < 0 || new_head.0 >= grid_size || new_head.1 < 0 || new_head.1 >= grid_size {
        game_over.set(true);
        return;
    }
    if current_snake.contains(&new_head) {
        game_over.set(true);
        return;
    }

    let mut next_snake = vec![new_head];
    next_snake.extend_from_slice(&current_snake);

    if new_head == **food {
        let points = if **is_gold { 30 } else { 10 };
        let new_score = **score + points;
        score.set(new_score);
        if new_score > **high_score {
            high_score.set(new_score);
            if let Some(win) = web_sys::window()
                && let Ok(Some(storage)) = win.local_storage()
            {
                let _ = storage.set_item("snake_high_score", &new_score.to_string());
            }
        }
        // Set gold status for next food (15% chance)
        let next_is_gold = js_sys::Math::random() < 0.15;
        is_gold.set(next_is_gold);
        food.set(generate_food());
    } else {
        next_snake.pop();
    }
    snake.set(next_snake);
}
