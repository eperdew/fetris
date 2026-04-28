//! Debug visual test bench.
//!
//! Reuses the live render pipeline by populating the same resources gameplay
//! uses, then driving effects via `GameEvent::LineClear` events and a
//! debug-only `state_overlay` override on `render_state_text`.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::app_state::AppState;
use crate::components::ActivePieceBundle;
use crate::data::{GameMode, Kind, PieceKind, PiecePhase};
use crate::judge::Judge;
use crate::resources::{
    Board, CurrentPhase, GameModeRes, GameProgress, NextPiece, PendingCompaction, RotationKind,
    RotationSystemRes,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum DebugStateOverlay {
    #[default]
    None,
    Ready,
    GameOver,
    Won,
}

#[derive(Resource, Default)]
pub struct DebugSceneState {
    pub hud_preset: usize,
    pub state_overlay: DebugStateOverlay,
    pub state_overlay_ticks_left: u32,
    pub line_clear_cleanup_ticks_left: u32,
}

/// Six presets covering Grade 9 → S9 with matching score/level.
/// Score values land at the lower bound of each grade band.
pub(crate) const HUD_PRESETS: &[(u32, u32)] = &[
    (0, 0),        // Grade 9
    (1700, 250),   // Grade 6
    (8500, 500),   // Grade 2
    (16000, 700),  // Grade S1
    (52000, 850),  // Grade S5
    (120000, 999), // Grade S9
];

pub(crate) fn apply_hud_preset(judge: &mut Judge, progress: &mut GameProgress, idx: usize) {
    let (score, level) = HUD_PRESETS[idx % HUD_PRESETS.len()];
    *judge = Judge::new();
    judge.set_score_for_debug(score);
    progress.level = level;
    progress.lines = level / 10;
    progress.ticks_elapsed = (level as u64) * 60;
    progress.initial_delay_ticks = 0;
    progress.game_over = false;
    progress.game_won = false;
}

pub fn on_enter_debug(world: &mut World) {
    world.insert_resource(RotationSystemRes(Kind::Ars.create()));
    world.insert_resource(GameModeRes(GameMode::Master));
    world.insert_resource(RotationKind(Kind::Ars));
    world.insert_resource(NextPiece(PieceKind::T));
    world.insert_resource(CurrentPhase(PiecePhase::Falling));
    world.insert_resource(DebugSceneState::default());

    let prior: Vec<Entity> = world
        .query::<(Entity, &crate::components::ActivePiece)>()
        .iter(world)
        .map(|(e, _)| e)
        .collect();
    for e in prior {
        world.despawn(e);
    }
    let mut bundle = ActivePieceBundle::new(PieceKind::T);
    bundle.position.row = 8;
    bundle.position.col = 4;
    world.spawn(bundle);

    world.resource_scope::<Judge, _>(|world, mut judge| {
        let mut progress = world.resource_mut::<GameProgress>();
        apply_hud_preset(&mut judge, &mut progress, 0);
    });
}

pub fn debug_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut scene: ResMut<DebugSceneState>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
    mut events: bevy::ecs::message::MessageWriter<crate::data::GameEvent>,
    mut judge: ResMut<Judge>,
    mut progress: ResMut<GameProgress>,
    mut next_state: ResMut<NextState<AppState>>,
    mut menu: ResMut<crate::menu::state::MenuState>,
) {
    if keys.just_pressed(KeyCode::Backspace) {
        menu.screen = crate::menu::state::MenuScreen::Main;
        next_state.set(AppState::Menu);
        return;
    }

    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::ArrowUp) {
        let delta: i32 = if keys.just_pressed(KeyCode::ArrowDown) {
            1
        } else {
            -1
        };
        let len = HUD_PRESETS.len() as i32;
        let new_idx = (scene.hud_preset as i32 + delta).rem_euclid(len) as usize;
        scene.hud_preset = new_idx;
        apply_hud_preset(&mut judge, &mut progress, new_idx);
    }

    if keys.just_pressed(KeyCode::KeyQ) {
        scene.state_overlay = DebugStateOverlay::Ready;
        scene.state_overlay_ticks_left = 90;
    } else if keys.just_pressed(KeyCode::KeyW) {
        scene.state_overlay = DebugStateOverlay::GameOver;
        scene.state_overlay_ticks_left = 90;
    } else if keys.just_pressed(KeyCode::KeyR) {
        scene.state_overlay = DebugStateOverlay::Won;
        scene.state_overlay_ticks_left = 90;
    }

    let count = if keys.just_pressed(KeyCode::Digit1) {
        Some(1u32)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(3)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(4)
    } else {
        None
    };

    if let Some(count) = count {
        if scene.line_clear_cleanup_ticks_left > 0 {
            return;
        }
        let n = count as usize;
        let rows: Vec<usize> = (crate::data::BOARD_ROWS - n..crate::data::BOARD_ROWS).collect();
        for &r in &rows {
            for c in 0..crate::data::BOARD_COLS {
                board.0[r][c] = Some(PieceKind::T);
            }
        }
        pending.0 = rows;
        events.write(crate::data::GameEvent::LineClear { count });
        scene.line_clear_cleanup_ticks_left = 3;
    }
}

pub fn debug_tick_system(
    mut scene: ResMut<DebugSceneState>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
    mut progress: ResMut<GameProgress>,
) {
    // The render pipeline drives shader effects (FETRIS hue-shift, scanline parity)
    // off `progress.ticks_elapsed`. Gameplay's `tick_counter` doesn't run in Debug,
    // so advance time here to keep the visuals animating.
    progress.ticks_elapsed = progress.ticks_elapsed.wrapping_add(1);

    if scene.line_clear_cleanup_ticks_left > 0 {
        scene.line_clear_cleanup_ticks_left -= 1;
        if scene.line_clear_cleanup_ticks_left == 0 {
            for r in &pending.0 {
                for c in 0..crate::data::BOARD_COLS {
                    board.0[*r][c] = None;
                }
            }
            pending.0.clear();
        }
    }

    if scene.state_overlay_ticks_left > 0 {
        scene.state_overlay_ticks_left -= 1;
        if scene.state_overlay_ticks_left == 0 {
            scene.state_overlay = DebugStateOverlay::None;
        }
    }
}

pub fn debug_keymap_panel(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::SidePanel::right("debug_keymap")
        .resizable(false)
        .default_width(220.0)
        .frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(10, 10, 18, 220)))
        .show(ctx, |ui| {
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("DEBUG")
                    .color(egui::Color32::WHITE)
                    .size(20.0),
            );
            ui.add_space(8.0);
            let row = |ui: &mut egui::Ui, k: &str, what: &str| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(k)
                            .color(egui::Color32::from_rgb(180, 180, 220))
                            .size(14.0),
                    );
                    ui.label(
                        egui::RichText::new(what)
                            .color(egui::Color32::GRAY)
                            .size(14.0),
                    );
                });
            };
            row(ui, "1 / 2 / 3 / 4", "line-clear bursts");
            row(ui, "Q", "READY");
            row(ui, "W", "GAME OVER");
            row(ui, "R", "LEVEL 999 win");
            row(ui, "↑ / ↓", "cycle HUD preset");
            row(ui, "Backspace", "back to menu");
        });
}
