//! Aggregated Snake game state and the [`use_snake_state`] hook.
//!
//! Sibling modules provide the focused effects and callbacks:
//! [`super::food`] (gold timer, `generate_food`),
//! [`super::tick`] (interval loop), [`super::actions`] (callbacks), and
//! [`super::keys`] (keyboard listener). This file keeps the hook body
//! readable by inlining the two small effects that don't warrant their
//! own module: leaderboard pre-fetch and footer score banner.

use crate::api::LeaderboardEntry;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use super::Pos;
use super::actions::{
    make_on_dpad_press, make_on_name_input, make_on_restart, make_on_resume, make_on_submit_score,
    make_set_next_dir,
};
use super::food::{install_gold_timeout, load_high_score};
use super::keys::install_keyboard_listener;
use super::tick::{TickInputs, install_tick_loop};

/// Bundle returned by [`use_snake_state`].
pub struct SnakeState {
    pub snake: UseStateHandle<Vec<Pos>>,
    pub food: UseStateHandle<Pos>,
    pub score: UseStateHandle<u32>,
    pub high_score: UseStateHandle<u32>,
    pub game_over: UseStateHandle<bool>,
    pub paused: UseStateHandle<bool>,
    pub started: UseStateHandle<bool>,
    pub is_gold: UseStateHandle<bool>,
    pub leaderboard: UseStateHandle<Vec<LeaderboardEntry>>,
    pub player_name: UseStateHandle<String>,
    pub submitting: UseStateHandle<bool>,
    pub on_restart: Callback<MouseEvent>,
    pub on_submit_score: Callback<SubmitEvent>,
    pub on_name_input: Callback<InputEvent>,
    pub on_resume: Callback<MouseEvent>,
    pub on_dpad_press: Callback<(i32, i32)>,
}

/// Top-level hook that wires the Snake game together.
#[hook]
pub fn use_snake_state(on_status: Callback<Option<(String, String)>>) -> SnakeState {
    let snake = use_state(|| vec![(10, 10), (10, 11), (10, 12)]);
    let direction = use_state(|| (0, -1));
    let next_direction = use_state(|| (0, -1));
    let food = use_state(|| (5, 5));
    let score = use_state(|| 0);
    let high_score = use_state(load_high_score);
    let game_over = use_state(|| false);
    let paused = use_state(|| false);
    let started = use_state(|| false);
    let is_gold = use_state(|| false);
    let leaderboard = use_state(Vec::<LeaderboardEntry>::new);
    let player_name = use_state(String::new);
    let submitting = use_state(|| false);
    // `LocaleContext` is always provided by the root `App` view. Documented
    // per the "no unwrap in non-test code" rule that applies to this crate.
    let locale =
        use_context::<crate::i18n::LocaleContext>().expect("LocaleContext provided by App view");

    // Pre-fetch the leaderboard on mount so the panel renders populated
    // data on first paint; later updates arrive via `on_submit_score`.
    {
        let leaderboard = leaderboard.clone();
        use_effect_with((), move |_| {
            let leaderboard = leaderboard.clone();
            spawn_local(async move {
                if let Ok(list) = crate::api::ApiService::get_leaderboard().await {
                    leaderboard.set(list);
                }
            });
            || ()
        });
    }

    // Footer banner: shows the live score, or the localised "game_over"
    // string on collision. The dep tuple is plain values, so the closure
    // captures nothing stateful from the surrounding scope.
    {
        let on_status = on_status.clone();
        let locale = locale.clone();
        use_effect_with((*score, *game_over), move |&(s, go)| {
            if go {
                on_status.emit(Some((locale.t("game_over"), "error".to_string())));
            } else {
                on_status.emit(Some((format!("Score: {}", s), "success".to_string())));
            }
            || ()
        });
    }

    install_gold_timeout(&is_gold, &food, &snake);
    install_tick_loop(TickInputs {
        started: started.clone(),
        paused: paused.clone(),
        game_over: game_over.clone(),
        score: score.clone(),
        direction: direction.clone(),
        next_direction: next_direction.clone(),
        snake: snake.clone(),
        food: food.clone(),
        high_score: high_score.clone(),
        is_gold: is_gold.clone(),
    });

    let on_restart = make_on_restart(
        &snake,
        &direction,
        &next_direction,
        &food,
        &score,
        &game_over,
        &paused,
        &started,
        &is_gold,
    );
    let on_submit_score = make_on_submit_score(&player_name, &score, &submitting, &leaderboard);
    let on_name_input = make_on_name_input(&player_name);
    let on_resume = make_on_resume(&paused);
    let set_next_dir = make_set_next_dir(&next_direction, &direction);
    let on_dpad_press = make_on_dpad_press(set_next_dir);
    install_keyboard_listener(&started, &game_over, &paused, &on_dpad_press);

    SnakeState {
        snake,
        food,
        score,
        high_score,
        game_over,
        paused,
        started,
        is_gold,
        leaderboard,
        player_name,
        submitting,
        on_restart,
        on_submit_score,
        on_name_input,
        on_resume,
        on_dpad_press,
    }
}
