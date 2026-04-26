use crate::components::{ActivePiece, PieceKindComp, PiecePosition, PieceRotation};
use crate::data::{PiecePhase, BOARD_COLS, BOARD_ROWS};
use crate::render::assets::GameAssets;
use crate::render::{cell_sprite, piece_color, BOARD_X, BOARD_Y, CELL, PAD};
use crate::resources::{Board, CurrentPhase, NextPiece, RotationSystemRes};
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
pub struct PieceSprite;

#[derive(Component, Clone, Copy)]
pub struct NextPreviewSprite;

pub fn render_active_piece(
    mut commands: Commands,
    existing: Query<Entity, With<PieceSprite>>,
    active: Query<(&PieceKindComp, &PiecePosition, &PieceRotation), With<ActivePiece>>,
    rotation_system: Res<RotationSystemRes>,
    assets: Res<GameAssets>,
    board: Res<Board>,
    phase: Res<CurrentPhase>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }

    // Don't render during LineClearDelay or Spawning — the piece has been locked
    // into the board and the next piece hasn't appeared yet.
    if !matches!(phase.0, PiecePhase::Falling | PiecePhase::Locking { .. }) {
        return;
    }

    let Ok((kind_comp, pos, rot)) = active.single() else {
        return;
    };
    let kind = kind_comp.0;
    let cells = rotation_system.0.cells(kind, rot.0);

    // Ghost.
    let mut ghost_row = pos.row;
    loop {
        let next_row = ghost_row + 1;
        if !can_place(&board.0, &cells, pos.col, next_row) {
            break;
        }
        ghost_row = next_row;
    }
    if ghost_row != pos.row {
        let base = piece_color(kind).to_srgba();
        let ghost_color = Color::srgba(base.red, base.green, base.blue, 0.25);
        for (dc, dr) in cells {
            let c = pos.col + dc;
            let r = ghost_row + dr;
            if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
                spawn_cell_sprite(&mut commands, &assets, c, r, ghost_color, 3.0);
            }
        }
    }

    // Active.
    let color = piece_color(kind);
    for (dc, dr) in cells {
        let c = pos.col + dc;
        let r = pos.row + dr;
        if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
            spawn_cell_sprite(&mut commands, &assets, c, r, color, 4.0);
        }
    }
}

pub fn render_next_preview(
    mut commands: Commands,
    existing: Query<Entity, With<NextPreviewSprite>>,
    next: Res<NextPiece>,
    rotation_system: Res<RotationSystemRes>,
    assets: Res<GameAssets>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }
    let kind = next.0;
    let cells = rotation_system.0.cells(kind, 0);
    let preview_y_offset = rotation_system.0.preview_y_offset(kind);
    let color = piece_color(kind);
    for (dc, dr) in cells {
        let c = 3 + dc;
        let r = -3 + dr + preview_y_offset;
        let x = BOARD_X + c as f32 * CELL;
        let y = (BOARD_Y - PAD) + r as f32 * CELL;
        commands.spawn((
            NextPreviewSprite,
            cell_sprite(x, y, color, assets.cell_texture.clone(), 5.0),
        ));
    }
}

fn spawn_cell_sprite(
    commands: &mut Commands,
    assets: &GameAssets,
    col: i32,
    row: i32,
    color: Color,
    z: f32,
) {
    let x = BOARD_X + col as f32 * CELL;
    let y = BOARD_Y + row as f32 * CELL;
    commands.spawn((
        PieceSprite,
        cell_sprite(x, y, color, assets.cell_texture.clone(), z),
    ));
}

fn can_place(board: &crate::data::BoardGrid, cells: &[(i32, i32); 4], col: i32, row: i32) -> bool {
    for &(dc, dr) in cells {
        let c = col + dc;
        let r = row + dr;
        if c < 0 || c >= BOARD_COLS as i32 {
            return false;
        }
        if r >= BOARD_ROWS as i32 {
            return false;
        }
        if r < 0 {
            continue;
        }
        if board[r as usize][c as usize].is_some() {
            return false;
        }
    }
    true
}
