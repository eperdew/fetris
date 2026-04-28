use crate::app_state::AppState;
use crate::data::{GameEvent, BOARD_COLS, BOARD_ROWS};
use crate::render::assets::GameAssets;
use crate::render::{BOARD_X, BOARD_Y, CELL, OVERLAY_RENDER_LAYER};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

const OVERLAY_LIFETIME: u32 = 45;

#[derive(Component)]
pub struct LineClearOverlay {
    pub ticks_left: u32,
    pub kind: OverlayKind,
}

#[derive(Clone, Copy)]
pub enum OverlayKind {
    Double,
    Triple,
    Fetris,
}

pub fn overlay_opacity(kind: OverlayKind) -> f32 {
    match kind {
        OverlayKind::Double => 0.45,
        OverlayKind::Triple => 0.75,
        OverlayKind::Fetris => 1.0,
    }
}

pub fn overlay_hue_shift(kind: OverlayKind, ticks_elapsed: u64) -> f32 {
    match kind {
        OverlayKind::Fetris => (ticks_elapsed as f32 * 0.03) % 1.0,
        _ => 0.0,
    }
}

/// Text color for the overlay label.
///
/// FETRIS uses a saturated red so the shader's hue-shift visibly cycles the
/// text through ROYGBIV; rotating pure white's hue is a no-op (white sits on
/// the achromatic axis).
fn overlay_text_color(kind: OverlayKind) -> Color {
    match kind {
        OverlayKind::Fetris => Color::srgba_u8(255, 60, 60, 255),
        OverlayKind::Double | OverlayKind::Triple => Color::WHITE,
    }
}

pub fn spawn_line_clear_overlay(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    existing: Query<Entity, With<LineClearOverlay>>,
    assets: Res<GameAssets>,
) {
    for ev in events.read() {
        let GameEvent::LineClear { count } = *ev else {
            continue;
        };
        let (label, kind) = match count {
            2 => ("DOUBLE", OverlayKind::Double),
            3 => ("TRIPLE", OverlayKind::Triple),
            4 => ("FETRIS", OverlayKind::Fetris),
            _ => continue,
        };

        for e in &existing {
            commands.entity(e).despawn();
        }

        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;

        commands.spawn((
            LineClearOverlay {
                ticks_left: OVERLAY_LIFETIME,
                kind,
            },
            Text2d::new(label),
            TextFont {
                font: assets.font.clone(),
                font_size: 40.0,
                ..default()
            },
            TextColor(overlay_text_color(kind)),
            bevy::sprite::Anchor::CENTER,
            Transform::from_xyz(cx, cy, 200.0).with_scale(Vec3::new(1.0, -1.0, 1.0)),
            RenderLayers::layer(OVERLAY_RENDER_LAYER),
        ));
    }
}

pub fn tick_line_clear_overlay(
    mut commands: Commands,
    mut q: Query<(Entity, &mut LineClearOverlay)>,
) {
    for (entity, mut o) in &mut q {
        if o.ticks_left == 0 {
            commands.entity(entity).despawn();
        } else {
            o.ticks_left -= 1;
        }
    }
}

#[derive(Component)]
pub struct StateText;

pub fn render_state_text(
    mut commands: Commands,
    existing: Query<Entity, With<StateText>>,
    state: Res<State<AppState>>,
    progress: Res<crate::resources::GameProgress>,
    assets: Res<GameAssets>,
    debug_scene: Option<Res<crate::menu::debug::DebugSceneState>>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }
    let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
    let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;

    let mk = |commands: &mut Commands, text: String, dy: f32, size: f32, color: Color| {
        commands.spawn((
            StateText,
            Text2d::new(text),
            TextFont {
                font: assets.font.clone(),
                font_size: size,
                ..default()
            },
            TextColor(color),
            bevy::sprite::Anchor::CENTER,
            Transform::from_xyz(cx, cy + dy, 150.0).with_scale(Vec3::new(1.0, -1.0, 1.0)),
        ));
    };

    if matches!(state.get(), AppState::Debug) {
        use crate::menu::debug::DebugStateOverlay;
        if let Some(scene) = debug_scene {
            match scene.state_overlay {
                DebugStateOverlay::Ready => {
                    mk(&mut commands, "READY".into(), 0.0, 28.0, Color::WHITE);
                }
                DebugStateOverlay::GameOver => {
                    mk(&mut commands, "GAME OVER".into(), 0.0, 28.0, Color::WHITE);
                }
                DebugStateOverlay::Won => {
                    mk(&mut commands, "LEVEL 999".into(), -16.0, 28.0, Color::WHITE);
                    mk(
                        &mut commands,
                        crate::render::hud::format_time(progress.ticks_elapsed),
                        20.0,
                        22.0,
                        Color::srgba(0.83, 0.83, 0.83, 1.0),
                    );
                }
                DebugStateOverlay::None => {}
            }
        }
        return;
    }

    match state.get() {
        AppState::Ready => {
            mk(&mut commands, "READY".into(), 0.0, 28.0, Color::WHITE);
        }
        AppState::Playing if progress.initial_delay_ticks > 0 => {
            mk(&mut commands, "READY".into(), 0.0, 28.0, Color::WHITE);
        }
        AppState::GameOver if progress.game_won => {
            mk(&mut commands, "LEVEL 999".into(), -16.0, 28.0, Color::WHITE);
            mk(
                &mut commands,
                crate::render::hud::format_time(progress.ticks_elapsed),
                20.0,
                22.0,
                Color::srgba(0.83, 0.83, 0.83, 1.0),
            );
        }
        AppState::GameOver => {
            mk(&mut commands, "GAME OVER".into(), 0.0, 28.0, Color::WHITE);
        }
        _ => {}
    }
}
