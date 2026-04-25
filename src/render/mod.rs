use bevy::prelude::*;

pub mod assets;
pub mod board;
pub mod hud;
pub mod overlays;
pub mod particles;
pub mod piece;

pub const CELL: f32 = 32.0;
pub const INSET: f32 = 2.0;
pub const PAD: f32 = 20.0;
pub const BOARD_X: f32 = PAD;
pub const BOARD_Y: f32 = 2.0 * CELL + 2.0 * PAD;
pub const BAR_WIDTH: f32 = 24.0;
pub const BAR_LEFT_GAP: f32 = 24.0;
pub const BAR_RIGHT_GAP: f32 = 14.0;
pub const BAR_X: f32 = BOARD_X + crate::data::BOARD_COLS as f32 * CELL + BAR_LEFT_GAP;
pub const SIDEBAR_X: f32 = BAR_X + BAR_WIDTH + BAR_RIGHT_GAP;
pub const DIVIDER_X: f32 = BOARD_X + crate::data::BOARD_COLS as f32 * CELL + BAR_LEFT_GAP / 2.0;
pub const WINDOW_W: f32 = 560.0;
pub const WINDOW_H: f32 = 780.0;
pub const BOARD_BG: Color = Color::srgba(0.06, 0.06, 0.10, 1.0);

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, assets::load_assets);
        // Tasks 4-6 add Update systems here.
    }
}

pub fn piece_color(kind: crate::data::PieceKind) -> Color {
    use crate::data::PieceKind;
    match kind {
        PieceKind::I => Color::srgba_u8(200, 50, 50, 255),
        PieceKind::O => Color::srgba_u8(220, 200, 0, 255),
        PieceKind::T => Color::srgba_u8(0, 200, 200, 255),
        PieceKind::S => Color::srgba_u8(200, 0, 200, 255),
        PieceKind::Z => Color::srgba_u8(0, 160, 0, 255),
        PieceKind::J => Color::srgba_u8(50, 100, 220, 255),
        PieceKind::L => Color::srgba_u8(255, 150, 100, 255),
    }
}

/// Returns a bundle: (Sprite with image+color+custom_size, Anchor::TOP_LEFT, Transform).
/// In Bevy 0.18, Anchor is a separate Component (not a field on Sprite).
pub fn cell_sprite(
    x: f32,
    y: f32,
    color: Color,
    texture: Handle<Image>,
    z: f32,
) -> impl Bundle {
    (
        Sprite {
            image: texture,
            color,
            custom_size: Some(Vec2::new(CELL - INSET * 2.0, CELL - INSET * 2.0)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(x + INSET, y + INSET, z),
    )
}
