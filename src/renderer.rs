use macroquad::prelude::*;
use crate::game::{BOARD_COLS, BOARD_ROWS, Game, PiecePhase};
use crate::piece::PieceKind;

const CELL: f32 = 32.0;
const PAD: f32 = 20.0;
const BOARD_X: f32 = PAD;
const BOARD_Y: f32 = PAD;
const SIDEBAR_X: f32 = BOARD_X + BOARD_COLS as f32 * CELL + 10.0;
const BOARD_BG: Color = Color::new(0.06, 0.06, 0.10, 1.0);

/// Draw a single CELL×CELL block at grid position (col, row) relative to (origin_x, origin_y).
fn draw_cell(origin_x: f32, origin_y: f32, col: usize, row: usize, color: Color) {
    const INSET: f32 = 2.0;
    draw_rectangle(
        origin_x + col as f32 * CELL + INSET,
        origin_y + row as f32 * CELL + INSET,
        CELL - INSET * 2.0,
        CELL - INSET * 2.0,
        color,
    );
}

fn piece_color(kind: PieceKind) -> Color {
    match kind {
        PieceKind::I => Color::from_rgba(200, 50,  50,  255),
        PieceKind::O => Color::from_rgba(220, 200, 0,   255),
        PieceKind::T => Color::from_rgba(0,   200, 200, 255),
        PieceKind::S => Color::from_rgba(200, 0,   200, 255),
        PieceKind::Z => Color::from_rgba(0,   160, 0,   255),
        PieceKind::J => Color::from_rgba(50,  100, 220, 255),
        PieceKind::L => Color::from_rgba(255, 150, 100, 255),
    }
}

pub fn format_time(ticks: u64) -> String {
    let seconds = ticks / 60;
    let ms = (ticks % 60) * 1000 / 60;
    let mm = seconds / 60;
    let ss = seconds % 60;
    format!("{:02}:{:02}.{:03}", mm, ss, ms)
}

fn render_board(game: &Game) {
    // Background
    draw_rectangle(
        BOARD_X, BOARD_Y,
        BOARD_COLS as f32 * CELL,
        BOARD_ROWS as f32 * CELL,
        BOARD_BG,
    );

    // Locked cells
    for (r, row) in game.board.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if let Some(kind) = cell {
                draw_cell(BOARD_X, BOARD_Y, c, r, piece_color(*kind));
            }
        }
    }

    // Active piece (hidden during spawn delay and line clear)
    if !matches!(
        game.piece_phase,
        PiecePhase::Spawning { .. } | PiecePhase::LineClearDelay { .. }
    ) {
        for (dc, dr) in game.active.cells() {
            let c = game.active.col + dc;
            let r = game.active.row + dr;
            if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
                draw_cell(BOARD_X, BOARD_Y, c as usize, r as usize, piece_color(game.active.kind));
            }
        }
    }
}

fn render_sidebar(game: &Game) {
    let x = SIDEBAR_X;
    let mut y = BOARD_Y + 16.0;

    draw_text("NEXT", x, y, 18.0, LIGHTGRAY);
    y += 8.0;

    for (dc, dr) in game.next.cells() {
        let c = dc as usize;
        let r = dr as usize;
        draw_cell(x, y, c, r, piece_color(game.next.kind));
    }
    y += 4.0 * CELL + 16.0;

    draw_text(&format!("LV  {}", game.level), x, y, 18.0, WHITE);
    y += 26.0;
    draw_text(&format!("LN  {}", game.lines), x, y, 18.0, WHITE);
    y += 26.0;
    draw_text(&format_time(game.ticks_elapsed), x, y, 18.0, WHITE);
}

fn render_overlay(game: &Game) {
    let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
    let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
    if game.game_won {
        draw_text("LEVEL 999", cx - 60.0, cy - 16.0, 28.0, WHITE);
        draw_text(&format_time(game.ticks_elapsed), cx - 50.0, cy + 20.0, 22.0, LIGHTGRAY);
    } else if game.game_over {
        draw_text("GAME OVER", cx - 62.0, cy, 28.0, WHITE);
    }
}

pub fn render(game: &Game) {
    clear_background(Color::from_rgba(10, 10, 18, 255));
    render_board(game);
    render_sidebar(game);
    render_overlay(game);
}
