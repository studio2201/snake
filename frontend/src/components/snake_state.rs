use yew::prelude::*;
use gloo_timers::callback::Interval;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use crate::api::{ApiService, LeaderboardEntry};
use super::snake_logic::handle_tick;

const GRID_SIZE: i32 = 20;

pub struct SnakeState {
    pub snake: UseStateHandle<Vec<(i32, i32)>>,
    pub food: UseStateHandle<(i32, i32)>,
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

#[hook]
pub fn use_snake_state(on_status: Callback<Option<(String, String)>>) -> SnakeState {
    let snake = use_state(|| vec![(10, 10), (10, 11), (10, 12)]);
    let direction = use_state(|| (0, -1)); // Up
    let next_direction = use_state(|| (0, -1));
    let food = use_state(|| (5, 5));
    let score = use_state(|| 0);
    let high_score = use_state(|| {
        if let Some(win) = web_sys::window()
            && let Ok(Some(storage)) = win.local_storage()
            && let Ok(Some(hs_val)) = storage.get_item("snake_high_score")
        {
            hs_val.parse::<u32>().unwrap_or(0)
        } else {
            0
        }
    });
    let game_over = use_state(|| false);
    let paused = use_state(|| false);
    let started = use_state(|| false);
    let is_gold = use_state(|| false);
    let leaderboard = use_state(|| Vec::<LeaderboardEntry>::new());
    let player_name = use_state(|| "".to_string());
    let submitting = use_state(|| false);
    let locale = use_context::<crate::i18n::LocaleContext>().unwrap();

    // Fetch leaderboard on load
    {
        let leaderboard = leaderboard.clone();
        use_effect_with((), move |_| {
            let leaderboard = leaderboard.clone();
            spawn_local(async move {
                if let Ok(list) = ApiService::get_leaderboard().await {
                    leaderboard.set(list);
                }
            });
            || ()
        });
    }

    // Set notification status
    {
        let on_status = on_status.clone();
        let score_val = *score;
        let game_over_val = *game_over;
        let locale = locale.clone();
        use_effect_with((score_val, game_over_val), move |&(s, go)| {
            if go {
                on_status.emit(Some((locale.t("game_over"), "error".to_string())));
            } else {
                on_status.emit(Some((format!("Score: {}", s), "success".to_string())));
            }
            || ()
        });
    }

    // Gold Food timeout hook
    {
        let is_gold = is_gold.clone();
        let food = food.clone();
        let snake = snake.clone();
        use_effect_with((*is_gold, *food), move |&(gold, _f)| {
            if !gold {
                return Box::new(|| ()) as Box<dyn FnOnce()>;
            }
            
            // Helper to generate food
            let generate_food = {
                let snake = snake.clone();
                move || {
                    let mut attempts = 0;
                    loop {
                        let x = (js_sys::Math::random() * GRID_SIZE as f64) as i32;
                        let y = (js_sys::Math::random() * GRID_SIZE as f64) as i32;
                        let on_snake = snake.iter().any(|&pos| pos == (x, y));
                        if !on_snake || attempts > 100 {
                            return (x, y);
                        }
                        attempts += 1;
                    }
                }
            };

            let is_gold = is_gold.clone();
            let food = food.clone();
            let timeout = gloo_timers::callback::Timeout::new(5000, move || {
                is_gold.set(false);
                food.set(generate_food());
            });
            Box::new(move || drop(timeout)) as Box<dyn FnOnce()>
        });
    }

    // Tick Interval Loop
    {
        let is_started = *started;
        let is_paused = *paused;
        let is_game_over = *game_over;
        let next_dir = next_direction.clone();
        let dir = direction.clone();
        let snake = snake.clone();
        let food = food.clone();
        let score = score.clone();
        let high_score = high_score.clone();
        let game_over = game_over.clone();
        let is_gold = is_gold.clone();
        let score_val = *score;

        // Helper to generate food
        let generate_food = {
            let snake = snake.clone();
            move || {
                let mut attempts = 0;
                loop {
                    let x = (js_sys::Math::random() * GRID_SIZE as f64) as i32;
                    let y = (js_sys::Math::random() * GRID_SIZE as f64) as i32;
                    let on_snake = snake.iter().any(|&pos| pos == (x, y));
                    if !on_snake || attempts > 100 {
                        return (x, y);
                    }
                    attempts += 1;
                }
            }
        };

        use_effect_with((is_started, is_paused, is_game_over, score_val), move |&(st, ps, go, s)| {
            if !st || ps || go {
                return Box::new(|| ()) as Box<dyn FnOnce()>;
            }
            let duration = std::cmp::max(75, 170 - (s as i32 / 20) * 15) as u32;
            let interval = Interval::new(duration, move || {
                handle_tick(
                    &snake,
                    &dir,
                    &next_dir,
                    &food,
                    &score,
                    &high_score,
                    &game_over,
                    &is_gold,
                    GRID_SIZE,
                    &generate_food,
                );
            });
            Box::new(move || drop(interval))
        });
    }

    // Helper to generate food for restart
    let generate_food_restart = {
        let snake = snake.clone();
        move || {
            let mut attempts = 0;
            loop {
                let x = (js_sys::Math::random() * GRID_SIZE as f64) as i32;
                let y = (js_sys::Math::random() * GRID_SIZE as f64) as i32;
                let on_snake = snake.iter().any(|&pos| pos == (x, y));
                if !on_snake || attempts > 100 {
                    return (x, y);
                }
                attempts += 1;
            }
        }
    };

    // Restart game callback
    let on_restart = {
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
            snake.set(vec![(10, 10), (10, 11), (10, 12)]);
            direction.set((0, -1));
            next_direction.set((0, -1));
            score.set(0);
            game_over.set(false);
            paused.set(false);
            started.set(true);
            is_gold.set(false);
            food.set(generate_food_restart());
        })
    };

    // Submit leaderboard score callback
    let on_submit_score = {
        let name = player_name.clone();
        let score_val = *score;
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
                if ApiService::submit_score(&name_str, score_val).await.is_ok() {
                    if let Ok(list) = ApiService::get_leaderboard().await {
                        leaderboard.set(list);
                    }
                }
                submitting.set(false);
            });
        })
    };

    let set_next_dir = {
        let next_dir = next_direction.clone();
        let dir = direction.clone();
        move |dx: i32, dy: i32| {
            let current_dir = *dir;
            if (dx != 0 && current_dir.0 == 0) || (dy != 0 && current_dir.1 == 0) {
                next_dir.set((dx, dy));
            }
        }
    };

    let on_name_input = {
        let player_name = player_name.clone();
        Callback::from(move |e: InputEvent| {
            let input = e.target_dyn_into::<HtmlInputElement>().unwrap();
            player_name.set(input.value());
        })
    };

    let on_resume = {
        let paused = paused.clone();
        Callback::from(move |_| paused.set(false))
    };

    let on_dpad_press = {
        let set_dir = set_next_dir.clone();
        Callback::from(move |(dx, dy)| set_dir(dx, dy))
    };

    // Register Keyboard Window Event Listener
    {
        let on_dpad_press = on_dpad_press.clone();
        let is_started = *started;
        let is_game_over = *game_over;
        let is_paused = *paused;
        let paused = paused.clone();
        use_effect_with((is_started, is_game_over, is_paused), move |&(st, go, ps)| {
            let window = web_sys::window().unwrap();
            let on_dpad_press = on_dpad_press.clone();
            let paused = paused.clone();
            let listener = crate::components::event_listener::EventListener::new(&window, "keydown", move |e: web_sys::Event| {
                let key_event = e.dyn_ref::<web_sys::KeyboardEvent>().unwrap();
                let key = key_event.key();
                
                // Toggle Pause
                if key == "Escape" || key == "p" || key == "P" {
                    if st && !go {
                        paused.set(!ps);
                    }
                    return;
                }

                // Disallow inputs if paused
                if ps {
                    return;
                }

                let direction = match key.as_str() {
                    "ArrowUp" | "w" | "W" => Some((0, -1)),
                    "ArrowDown" | "s" | "S" => Some((0, 1)),
                    "ArrowLeft" | "a" | "A" => Some((-1, 0)),
                    "ArrowRight" | "d" | "D" => Some((1, 0)),
                    _ => None,
                };
                if let Some(dir) = direction {
                    on_dpad_press.emit(dir);
                }
            });
            move || drop(listener)
        });
    }

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
