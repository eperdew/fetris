use bevy::prelude::*;
use crate::render::{BOARD_BG, BOARD_X, BOARD_Y, CELL, INSET, cell_sprite, piece_color};
use crate::render::assets::GameAssets;
use crate::data::{BOARD_COLS, BOARD_ROWS};
use crate::resources::{Board, PendingCompaction};

#[derive(Component, Clone, Copy)]
pub struct BoardSprite;

pub fn render_board(
    mut commands: Commands,
    existing: Query<Entity, With<BoardSprite>>,
    board: Res<Board>,
    pending: Res<PendingCompaction>,
    assets: Res<GameAssets>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }

    // Background.
    commands.spawn((
        BoardSprite,
        Sprite {
            color: BOARD_BG,
            custom_size: Some(Vec2::new(BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(BOARD_X, BOARD_Y, 0.0),
    ));

    // Locked cells (skip rows pending compaction).
    for r in 0..BOARD_ROWS {
        if pending.0.contains(&r) {
            continue;
        }
        for c in 0..BOARD_COLS {
            if let Some(kind) = board.0[r][c] {
                let left = c == 0 || board.0[r][c - 1].is_none();
                let top = r == 0 || board.0[r - 1][c].is_none();
                let right = c == BOARD_COLS - 1 || board.0[r][c + 1].is_none();
                let bottom = r == BOARD_ROWS - 1 || board.0[r + 1][c].is_none();
                spawn_bordered_cell(
                    &mut commands, &assets, c as i32, r as i32, piece_color(kind),
                    left, top, right, bottom,
                );
            }
        }
    }

    // Top dim overlay.
    commands.spawn((
        BoardSprite,
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 0.1),
            custom_size: Some(Vec2::new(BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(BOARD_X, BOARD_Y, 100.0),
    ));
}

fn spawn_bordered_cell(
    commands: &mut Commands,
    assets: &GameAssets,
    col: i32,
    row: i32,
    color: Color,
    left: bool,
    top: bool,
    right: bool,
    bottom: bool,
) {
    const BORDER: Color = Color::srgba(0.70, 0.70, 0.70, 1.0);
    let x = BOARD_X + col as f32 * CELL;
    let y = BOARD_Y + row as f32 * CELL;
    let mk_strip = |x: f32, y: f32, w: f32, h: f32| -> (BoardSprite, Sprite, bevy::sprite::Anchor, Transform) {
        (
            BoardSprite,
            Sprite {
                color: BORDER,
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(x, y, 1.0),
        )
    };
    if left   { commands.spawn(mk_strip(x, y, INSET, CELL)); }
    if top    { commands.spawn(mk_strip(x, y, CELL, INSET)); }
    if right  { commands.spawn(mk_strip(x + CELL - INSET, y, INSET, CELL)); }
    if bottom { commands.spawn(mk_strip(x, y + CELL - INSET, CELL, INSET)); }

    commands.spawn((BoardSprite, cell_sprite(x, y, color, assets.cell_texture.clone(), 2.0)));
}
