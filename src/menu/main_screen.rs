use crate::data::{GameMode, Kind};
use crate::menu::state::{MenuScreen, MenuState};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_pkv::PkvStore;

fn make_bracketed(label: &str, active: bool, size: f32) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    let bg = egui::Color32::from_rgb(10, 10, 18);
    let bracket_color = if active { egui::Color32::WHITE } else { bg };
    let font_id = egui::FontId::proportional(size);
    let mut job = LayoutJob::default();
    job.append(
        "< ",
        0.0,
        TextFormat {
            color: bracket_color,
            font_id: font_id.clone(),
            ..Default::default()
        },
    );
    job.append(
        label,
        0.0,
        TextFormat {
            color: egui::Color32::WHITE,
            font_id: font_id.clone(),
            ..Default::default()
        },
    );
    job.append(
        " >",
        0.0,
        TextFormat {
            color: bracket_color,
            font_id,
            ..Default::default()
        },
    );
    job
}

pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    pub back: bool,
    pub unlock_debug: bool,
}

pub fn read_input(keys: &ButtonInput<KeyCode>) -> MenuInput {
    MenuInput {
        up: keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyK),
        down: keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyJ),
        left: keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyH),
        right: keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyL),
        confirm: keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter),
        back: keys.just_pressed(KeyCode::Backspace),
        unlock_debug: keys.just_pressed(KeyCode::KeyD),
    }
}

pub fn main_menu_system(
    mut contexts: EguiContexts,
    mut menu: ResMut<MenuState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<crate::app_state::AppState>>,
    mut pkv: ResMut<PkvStore>,
) {
    if menu.screen != MenuScreen::Main {
        return;
    }
    let input = read_input(&keys);

    if input.unlock_debug {
        menu.debug_unlocked = true;
    }

    if input.up {
        menu.cursor = menu.cursor.saturating_sub(1);
    }
    let cursor_max = if menu.debug_unlocked { 5 } else { 4 };
    if input.down {
        menu.cursor = (menu.cursor + 1).min(cursor_max);
    }

    let mut start_game = false;
    let mut enter_debug = false;
    match menu.cursor {
        0 if input.left || input.right => {
            menu.game_mode = match menu.game_mode {
                GameMode::Master => GameMode::TwentyG,
                GameMode::TwentyG => GameMode::Master,
            };
        }
        1 if input.left || input.right => {
            menu.rotation = match menu.rotation {
                Kind::Ars => Kind::Srs,
                Kind::Srs => Kind::Ars,
            };
        }
        2 if input.confirm => {
            menu.hi_scores_tab = match (menu.game_mode, menu.rotation) {
                (GameMode::Master, Kind::Ars) => 0,
                (GameMode::Master, Kind::Srs) => 1,
                (GameMode::TwentyG, Kind::Ars) => 2,
                (GameMode::TwentyG, Kind::Srs) => 3,
            };
            menu.screen = MenuScreen::HiScores;
        }
        3 if input.confirm => {
            menu.screen = MenuScreen::Controls;
        }
        4 if input.confirm => {
            if menu.debug_unlocked {
                enter_debug = true;
            } else {
                start_game = true;
            }
        }
        5 if input.confirm => {
            start_game = true;
        }
        _ => {}
    }

    let ctx = contexts.ctx_mut().expect("egui context");
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(egui::Color32::from_rgb(10, 10, 18)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                const CONTENT_HEIGHT: f32 = 360.0;
                ui.add_space(((ui.available_height() - CONTENT_HEIGHT) / 2.0).max(20.0));
                let mode_str = match menu.game_mode {
                    GameMode::Master => "MASTER",
                    GameMode::TwentyG => "20G",
                };
                let rot_str = match menu.rotation {
                    Kind::Ars => "ARS",
                    Kind::Srs => "SRS",
                };
                let row = |ui: &mut egui::Ui, label: &str, color: egui::Color32, size: f32| {
                    ui.label(egui::RichText::new(label).color(color).size(size));
                };

                row(ui, "GAME MODE", egui::Color32::GRAY, 18.0);
                ui.label(make_bracketed(mode_str, menu.cursor == 0, 24.0));
                ui.add_space(20.0);
                row(ui, "ROTATION", egui::Color32::GRAY, 18.0);
                ui.label(make_bracketed(rot_str, menu.cursor == 1, 24.0));
                ui.add_space(20.0);
                ui.label(make_bracketed("HI SCORES", menu.cursor == 2, 24.0));
                ui.label(make_bracketed("CONTROLS", menu.cursor == 3, 24.0));
                if menu.debug_unlocked {
                    ui.label(make_bracketed("DEBUG", menu.cursor == 4, 24.0));
                }
                ui.add_space(20.0);
                let start_idx = if menu.debug_unlocked { 5 } else { 4 };
                ui.label(make_bracketed("START", menu.cursor == start_idx, 24.0));
                ui.add_space(60.0);
                let (label, color) = if pkv.get::<bool>("muted").unwrap_or(false) {
                    ("[M]  MUTED", egui::Color32::from_rgb(204, 102, 102))
                } else {
                    ("[M]  SOUND ON", egui::Color32::GRAY)
                };
                row(ui, label, color, 14.0);
                ui.add_space(8.0);
                row(
                    ui,
                    env!("GIT_HASH"),
                    egui::Color32::from_rgb(50, 50, 60),
                    11.0,
                );
            });
        });

    if enter_debug {
        next_state.set(crate::app_state::AppState::Debug);
    }

    if start_game {
        let _ = pkv.set(
            "game_config",
            &crate::data::GameConfig {
                game_mode: menu.game_mode,
                rotation: menu.rotation,
            },
        );
        next_state.set(crate::app_state::AppState::Ready);
    }
}
