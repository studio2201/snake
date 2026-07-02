use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SnakeBoardProps {
    pub snake: Vec<(i32, i32)>,
    pub food: (i32, i32),
    pub grid_size: i32,
    pub is_gold: bool,
}

#[function_component(SnakeBoard)]
pub fn snake_board(props: &SnakeBoardProps) -> Html {
    let grid_size = props.grid_size;
    let snake = &props.snake;
    let food = props.food;

    html! {
        <div class="game-grid">
            {
                for (0..grid_size).map(|y| {
                    html! {
                        <div class="grid-row" key={y}>
                            {
                                for (0..grid_size).map(|x| {
                                    let is_snake_head = snake[0] == (x, y);
                                    let is_snake_body = !is_snake_head && snake.iter().any(|&pos| pos == (x, y));
                                    let is_food = food == (x, y);

                                    let cell_class = if is_snake_head {
                                        "grid-cell snake-head"
                                    } else if is_snake_body {
                                        "grid-cell snake-body"
                                    } else if is_food {
                                        if props.is_gold {
                                            "grid-cell food gold-food"
                                        } else {
                                            "grid-cell food"
                                        }
                                    } else {
                                        "grid-cell"
                                    };

                                    html! { <div class={cell_class} key={x}></div> }
                                })
                            }
                        </div>
                    }
                })
            }
        </div>
    }
}
