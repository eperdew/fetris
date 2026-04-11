use crate::constants::{LINE_CLEAR_DELAY, PARTICLE_GRAVITY, PARTICLE_INITIAL_SPEED};
use crate::game::{BOARD_COLS, BOARD_ROWS, Game, PiecePhase};
use crate::menu::{GameMode, Menu, MenuScreen};
use crate::piece::PieceKind;
use crate::rotation_system::RotationSystem;
use macroquad::prelude::*;

const CELL: f32 = 32.0;
const INSET: f32 = 2.0;
const PAD: f32 = 20.0;
const BOARD_X: f32 = PAD;
const BOARD_Y: f32 = PAD;
const SIDEBAR_X: f32 = BOARD_X + BOARD_COLS as f32 * CELL + 10.0;
const BOARD_BG: Color = Color::new(0.06, 0.06, 0.10, 1.0);

pub fn make_cell_texture() -> Texture2D {
    const SIZE: usize = 32;
    let mut pixels = [255u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let fy = y as f32 / (SIZE - 1) as f32;

            let raw = if x == 0 || y == 0 {
                1.0
            } else {
                1.0 - 0.4 * fy
            };
            let quantized = (raw * 16.0).floor() / 16.0;
            let v = (quantized * 255.0) as u8;
            let i = (y * SIZE + x) * 4;
            pixels[i] = v;
            pixels[i + 1] = v;
            pixels[i + 2] = v;
            // alpha channel stays 255
        }
    }
    Texture2D::from_rgba8(SIZE as u16, SIZE as u16, &pixels)
}

/// Like draw_cell but draws white-grey border strips on the left and/or top edges when
/// the adjacent cell in that direction is unfilled.
fn draw_cell_bordered(
    origin_x: f32,
    origin_y: f32,
    col: usize,
    row: usize,
    color: Color,
    texture: &Texture2D,
    left_border: bool,
    top_border: bool,
    right_border: bool,
    bottom_border: bool,
) {
    const BORDER_COLOR: Color = Color::new(0.70, 0.70, 0.70, 1.0);
    let x = origin_x + col as f32 * CELL;
    let y = origin_y + row as f32 * CELL;
    if left_border {
        draw_rectangle(x, y, INSET, CELL, BORDER_COLOR);
    }
    if top_border {
        draw_rectangle(x, y, CELL, INSET, BORDER_COLOR);
    }
    if right_border {
        draw_rectangle(x + CELL - INSET, y, INSET, CELL, BORDER_COLOR);
    }
    if bottom_border {
        draw_rectangle(x, y + CELL - INSET, CELL, INSET, BORDER_COLOR);
    }
    draw_cell(origin_x, origin_y, col, row, color, texture);
}

/// Draw a single CELL×CELL block at pixel position (x, y).
fn draw_cell_at(x: f32, y: f32, color: Color, texture: &Texture2D) {
    draw_texture_ex(
        texture,
        x + INSET,
        y + INSET,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(CELL - INSET * 2.0, CELL - INSET * 2.0)),
            ..Default::default()
        },
    );
}

/// Draw a single CELL×CELL block at grid position (col, row) relative to (origin_x, origin_y).
fn draw_cell(
    origin_x: f32,
    origin_y: f32,
    col: usize,
    row: usize,
    color: Color,
    texture: &Texture2D,
) {
    draw_cell_at(
        origin_x + col as f32 * CELL,
        origin_y + row as f32 * CELL,
        color,
        texture,
    );
}

fn piece_color(kind: PieceKind) -> Color {
    match kind {
        PieceKind::I => Color::from_rgba(200, 50, 50, 255),
        PieceKind::O => Color::from_rgba(220, 200, 0, 255),
        PieceKind::T => Color::from_rgba(0, 200, 200, 255),
        PieceKind::S => Color::from_rgba(200, 0, 200, 255),
        PieceKind::Z => Color::from_rgba(0, 160, 0, 255),
        PieceKind::J => Color::from_rgba(50, 100, 220, 255),
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

fn compute_ghost_row(game: &Game) -> i32 {
    let mut ghost_row = game.active.row;
    loop {
        let next = ghost_row + 1;
        let blocked = game.active.cells().iter().any(|&(dc, dr)| {
            let c = game.active.col + dc;
            let r = next + dr;
            r >= BOARD_ROWS as i32
                || (c >= 0
                    && c < BOARD_COLS as i32
                    && r >= 0
                    && game.board[r as usize][c as usize].is_some())
        });
        if blocked {
            break;
        }
        ghost_row = next;
    }
    ghost_row
}

fn render_board(game: &Game, texture: &Texture2D) {
    // Background
    draw_rectangle(
        BOARD_X,
        BOARD_Y,
        BOARD_COLS as f32 * CELL,
        BOARD_ROWS as f32 * CELL,
        BOARD_BG,
    );

    let show_active = !matches!(
        game.piece_phase,
        PiecePhase::Spawning { .. } | PiecePhase::LineClearDelay { .. }
    );

    // Ghost piece
    if show_active {
        let ghost_row = compute_ghost_row(game);
        if ghost_row != game.active.row {
            let base = piece_color(game.active.kind);
            let ghost_color = Color { a: 0.25, ..base };
            for (dc, dr) in game.active.cells() {
                let c = game.active.col + dc;
                let r = ghost_row + dr;
                if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
                    draw_cell(
                        BOARD_X,
                        BOARD_Y,
                        c as usize,
                        r as usize,
                        ghost_color,
                        texture,
                    );
                }
            }
        }
    }

    // Locked cells (skip rows pending compaction — they are drawn as particles below)
    for (r, row) in game.board.iter().enumerate() {
        if game.rows_pending_compaction.contains(&r) {
            continue;
        }
        for (c, cell) in row.iter().enumerate() {
            if let Some(kind) = cell {
                let left_border = c == 0 || game.board[r][c - 1].is_none();
                let top_border = r == 0 || game.board[r - 1][c].is_none();
                let right_border = c == BOARD_COLS - 1 || game.board[r][c + 1].is_none();
                let bottom_border = r == BOARD_ROWS - 1 || game.board[r + 1][c].is_none();
                draw_cell_bordered(
                    BOARD_X,
                    BOARD_Y,
                    c,
                    r,
                    piece_color(*kind),
                    texture,
                    left_border,
                    top_border,
                    right_border,
                    bottom_border,
                );
            }
        }
    }

    // Particles: cells from cleared rows fly off screen during LineClearDelay
    if let PiecePhase::LineClearDelay { ticks_left } = game.piece_phase {
        let t = (LINE_CLEAR_DELAY - ticks_left) as f32;
        for &r in &game.rows_pending_compaction {
            for (c, cell) in game.board[r].iter().enumerate() {
                if let Some(kind) = cell {
                    let initial_x = BOARD_X + c as f32 * CELL;
                    let initial_y = BOARD_Y + r as f32 * CELL;
                    let dist = c as f32 - (BOARD_COLS as f32 - 1.0) / 2.0;
                    let height = (BOARD_ROWS - r) as f32 / BOARD_ROWS as f32;
                    let vx_raw = dist * height;
                    let vy_raw = (r + 1) as f32 / BOARD_ROWS as f32;
                    let len = (vx_raw * vx_raw + vy_raw * vy_raw).sqrt();
                    let vx = vx_raw / len * PARTICLE_INITIAL_SPEED;
                    let vy = vy_raw / len * PARTICLE_INITIAL_SPEED;
                    let px = initial_x + vx * t;
                    let py = initial_y + vy * t + 0.5 * PARTICLE_GRAVITY * t * t;
                    if px > -CELL && px < screen_width() && py > -CELL && py < screen_height() {
                        draw_cell_at(px, py, piece_color(*kind), texture);
                    }
                }
            }
        }
    }

    // Active piece
    if show_active {
        for (dc, dr) in game.active.cells() {
            let c = game.active.col + dc;
            let r = game.active.row + dr;
            if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
                draw_cell(
                    BOARD_X,
                    BOARD_Y,
                    c as usize,
                    r as usize,
                    piece_color(game.active.kind),
                    texture,
                );
            }
        }
    }
}

fn render_sidebar(game: &Game, texture: &Texture2D) {
    let x = SIDEBAR_X;
    let mut y = BOARD_Y + 16.0;

    draw_text("NEXT", x, y, 18.0, LIGHTGRAY);
    y += 8.0;

    for (dc, dr) in game.next.cells() {
        let c = dc as usize;
        let r = dr as usize;
        draw_cell(x, y, c, r, piece_color(game.next.kind), texture);
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
        draw_text(
            &format_time(game.ticks_elapsed),
            cx - 50.0,
            cy + 20.0,
            22.0,
            LIGHTGRAY,
        );
    } else if game.game_over {
        draw_text("GAME OVER", cx - 62.0, cy, 28.0, WHITE);
    }
}

pub fn render(game: &Game, texture: &Texture2D) {
    clear_background(Color::from_rgba(10, 10, 18, 255));
    render_board(game, texture);
    render_sidebar(game, texture);
    render_overlay(game);
}

fn maybe_bracket(s: &str, active: bool) -> String {
    if active {
        format!("< {} >", s)
    } else {
        format!("  {}  ", s)
    }
}

fn draw_centered(text: &str, y: f32, font_size: f32, color: Color) {
    let dims = measure_text(text, None, font_size as u16, 1.0);
    draw_text(
        text,
        (screen_width() - dims.width) / 2.0,
        y,
        font_size,
        color,
    );
}

fn render_main_menu(menu: &Menu) {
    const FONT: f32 = 24.0;
    const LH: f32 = 36.0;

    let mode_str = match menu.game_mode() {
        GameMode::Master => "MASTER",
        GameMode::TwentyG => "20G",
    };
    let rot_str = match menu.rotation() {
        RotationSystem::Ars => "ARS",
        RotationSystem::Srs => "SRS",
    };

    let mode_label = maybe_bracket(mode_str, menu.cursor() == 0);
    let rot_label = maybe_bracket(rot_str, menu.cursor() == 1);
    let hi_label = maybe_bracket("HI SCORES", menu.cursor() == 2);
    let ctrl_label = maybe_bracket("CONTROLS", menu.cursor() == 3);
    let start_label = maybe_bracket("START", menu.cursor() == 4);

    let lines: &[Option<(&str, Color)>] = &[
        Some(("GAME MODE", GRAY)),
        Some((&mode_label, WHITE)),
        None,
        Some(("ROTATION", GRAY)),
        Some((&rot_label, WHITE)),
        None,
        Some((&hi_label, WHITE)),
        Some((&ctrl_label, WHITE)),
        None,
        Some((&start_label, WHITE)),
    ];

    let total_h = lines.len() as f32 * LH;
    let start_y = (screen_height() - total_h) / 2.0 + LH;

    for (i, line) in lines.iter().enumerate() {
        if let Some((text, color)) = line {
            draw_centered(text, start_y + i as f32 * LH, FONT, *color);
        }
    }
}

fn render_subscreen(title: &str) {
    const FONT: f32 = 24.0;
    let cy = screen_height() / 2.0;
    draw_centered(title, cy - 20.0, FONT, WHITE);
    draw_centered("BKSP to go back", cy + 20.0, 18.0, GRAY);
}

pub fn render_menu(menu: &Menu) {
    clear_background(Color::from_rgba(10, 10, 18, 255));
    match menu.screen() {
        MenuScreen::Main => render_main_menu(menu),
        MenuScreen::HiScores => render_subscreen("HI SCORES"),
        MenuScreen::Controls => render_subscreen("CONTROLS"),
    }
}
