use yew::UseStateHandle;

pub fn handle_tick(
    snake: &UseStateHandle<Vec<(i32, i32)>>,
    dir: &UseStateHandle<(i32, i32)>,
    next_dir: &UseStateHandle<(i32, i32)>,
    food: &UseStateHandle<(i32, i32)>,
    score: &UseStateHandle<u32>,
    high_score: &UseStateHandle<u32>,
    game_over: &UseStateHandle<bool>,
    is_gold: &UseStateHandle<bool>,
    grid_size: i32,
    generate_food: &impl Fn() -> (i32, i32),
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
    if current_snake.iter().any(|&pos| pos == new_head) {
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
