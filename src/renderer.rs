use macroquad::prelude::*;
use crate::game::Game;

pub fn render(_game: &Game) {
    clear_background(BLACK);
}

// Used in Task 2 full renderer; suppress warning during stub phase.
#[allow(dead_code)]
pub fn format_time(ticks: u64) -> String {
    let seconds = ticks / 60;
    let ms = (ticks % 60) * 1000 / 60;
    let mm = seconds / 60;
    let ss = seconds % 60;
    format!("{:02}:{:02}.{:03}", mm, ss, ms)
}
