use yew::prelude::*;

use super::snake_board::SnakeBoard;
use super::snake_dpad::MobileDpad;
use super::snake_leaderboard::LeaderboardPanel;
use super::snake_overlay::SnakeOverlay;
use super::snake_state::use_snake_state;

const GRID_SIZE: i32 = 20;

#[derive(Properties, PartialEq)]
pub struct SnakeGameProps {
    pub on_status: Callback<Option<(String, String)>>,
}

#[function_component(SnakeGame)]
pub fn snake_game(props: &SnakeGameProps) -> Html {
    let state = use_snake_state(props.on_status.clone());
    let locale = use_context::<crate::i18n::LocaleContext>().unwrap();

    html! {
        <div class="snake-container">
            <div class="score-board">
                <div class="score-stat">
                    <span class="label">{format!("{}:", locale.t("score"))}</span>
                    <span class="value">{*state.score}</span>
                </div>
                <div class="score-stat">
                    <span class="label">{format!("{}:", locale.t("high_score"))}</span>
                    <span class="value">{*state.high_score}</span>
                </div>
            </div>

            <div class="board-relative-wrapper">
                <SnakeBoard snake={(*state.snake).clone()} food={*state.food} grid_size={GRID_SIZE} />
                <SnakeOverlay
                    started={*state.started}
                    game_over={*state.game_over}
                    paused={*state.paused}
                    score={*state.score}
                    submitting={*state.submitting}
                    player_name={(*state.player_name).clone()}
                    on_restart={state.on_restart.clone()}
                    on_submit_score={state.on_submit_score.clone()}
                    on_name_input={state.on_name_input.clone()}
                    on_resume={state.on_resume.clone()}
                />
            </div>

            <MobileDpad on_press={state.on_dpad_press.clone()} />

            <LeaderboardPanel leaderboard={(*state.leaderboard).clone()} />
        </div>
    }
}
