//! User-facing callbacks for the Snake game.
//!
//! Each helper here returns a single [`Callback`] ready to be passed into
//! the view layer. Keeping the helpers small and focused lets the
//! [`state`] hook body stay readable.

use super::Pos;
use super::food::generate_food;
use crate::api::{ApiService, LeaderboardEntry};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

/// Initial snake body used when a game starts or restarts.
const FRESH_SNAKE: [Pos; 3] = [(10, 10), (10, 11), (10, 12)];
/// Initial direction at the start of every run.
const INITIAL_DIR: Pos = (0, -1);

/// Builds the "PRESS START" / "PLAY AGAIN" callback.
#[allow(clippy::too_many_arguments)]
pub fn make_on_restart(
    snake: &UseStateHandle<Vec<Pos>>,
    direction: &UseStateHandle<Pos>,
    next_direction: &UseStateHandle<Pos>,
    food: &UseStateHandle<Pos>,
    score: &UseStateHandle<u32>,
    game_over: &UseStateHandle<bool>,
    paused: &UseStateHandle<bool>,
    started: &UseStateHandle<bool>,
    is_gold: &UseStateHandle<bool>,
) -> Callback<MouseEvent> {
    let snake = snake.clone();
    let direction = direction.clone();
    let next_direction = next_direction.clone();
    let food = food.clone();
    let score = score.clone();
    let game_over = game_over.clone();
    let paused = paused.clone();
    let started = started.clone();
    let is_gold = is_gold.clone();
    Callback::from(move |_| {
        snake.set(FRESH_SNAKE.to_vec());
        direction.set(INITIAL_DIR);
        next_direction.set(INITIAL_DIR);
        score.set(0);
        game_over.set(false);
        paused.set(false);
        started.set(true);
        is_gold.set(false);
        food.set(generate_food(&FRESH_SNAKE));
    })
}

/// Builds the leaderboard submission callback used by the game-over form.
pub fn make_on_submit_score(
    player_name: &UseStateHandle<String>,
    score: &UseStateHandle<u32>,
    submitting: &UseStateHandle<bool>,
    leaderboard: &UseStateHandle<Vec<LeaderboardEntry>>,
) -> Callback<SubmitEvent> {
    let name = player_name.clone();
    let score_val = **score;
    let submitting = submitting.clone();
    let leaderboard = leaderboard.clone();
    Callback::from(move |e: SubmitEvent| {
        e.prevent_default();
        let name_str = (*name).clone();
        if name_str.trim().is_empty() || *submitting {
            return;
        }
        submitting.set(true);
        let submitting = submitting.clone();
        let leaderboard = leaderboard.clone();
        spawn_local(async move {
            if ApiService::submit_score(&name_str, score_val).await.is_ok()
                && let Ok(list) = ApiService::get_leaderboard().await
            {
                leaderboard.set(list);
            }
            submitting.set(false);
        });
    })
}

/// Builds the input handler for the player-name field.
pub fn make_on_name_input(player_name: &UseStateHandle<String>) -> Callback<InputEvent> {
    let player_name = player_name.clone();
    Callback::from(move |e: InputEvent| {
        // The overlay markup binds `oninput` to an `<input type="text">`
        // element, so the cast cannot fail in practice.
        let input: HtmlInputElement = e.target_unchecked_into();
        player_name.set(input.value());
    })
}

/// Builds the "RESUME" callback used by the pause overlay.
pub fn make_on_resume(paused: &UseStateHandle<bool>) -> Callback<MouseEvent> {
    let paused = paused.clone();
    Callback::from(move |_| paused.set(false))
}

/// Returns a closure that applies a direction change, rejecting any
/// 180° reversal (which would cause an immediate self-collision).
pub fn make_set_next_dir(
    next_direction: &UseStateHandle<Pos>,
    direction: &UseStateHandle<Pos>,
) -> impl Fn(i32, i32) + Clone + use<> {
    let next_dir = next_direction.clone();
    let dir = direction.clone();
    move |dx: i32, dy: i32| {
        let current_dir = *dir;
        if (dx != 0 && current_dir.0 == 0) || (dy != 0 && current_dir.1 == 0) {
            next_dir.set((dx, dy));
        }
    }
}

/// Wraps [`make_set_next_dir`] in a Yew [`Callback`] for the mobile dpad.
pub fn make_on_dpad_press(set_next_dir: impl Fn(i32, i32) + 'static) -> Callback<(i32, i32)> {
    Callback::from(move |(dx, dy)| set_next_dir(dx, dy))
}
