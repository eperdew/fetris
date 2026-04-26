use crate::data::{GameMode, Kind};
use crate::menu::state::{MenuScreen, MenuState};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_pkv::PkvStore;

pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    pub back: bool,
}

pub fn read_input(keys: &ButtonInput<KeyCode>) -> MenuInput {
    MenuInput {
        up: keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyK),
        down: keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyJ),
        left: keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyH),
        right: keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyL),
        confirm: keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter),
        back: keys.just_pressed(KeyCode::Backspace),
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

    if input.up {
        menu.cursor = menu.cursor.saturating_sub(1);
    }
    if input.down {
        menu.cursor = (menu.cursor + 1).min(4);
    }

    let mut start_game = false;
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
            start_game = true;
        }
        _ => {}
    }

    let ctx = contexts.ctx_mut().expect("egui context");
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(egui::Color32::from_rgb(10, 10, 18)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(120.0);
                let mode_str = match menu.game_mode {
                    GameMode::Master => "MASTER",
                    GameMode::TwentyG => "20G",
                };
                let rot_str = match menu.rotation {
                    Kind::Ars => "ARS",
                    Kind::Srs => "SRS",
                };
                let bracket = |s: &str, active: bool| -> String {
                    if active {
                        format!("< {} >", s)
                    } else {
                        format!("  {}  ", s)
                    }
                };
                let row = |ui: &mut egui::Ui, label: &str, color: egui::Color32, size: f32| {
                    ui.label(egui::RichText::new(label).color(color).size(size));
                };

                row(ui, "GAME MODE", egui::Color32::GRAY, 18.0);
                row(
                    ui,
                    &bracket(mode_str, menu.cursor == 0),
                    egui::Color32::WHITE,
                    24.0,
                );
                ui.add_space(20.0);
                row(ui, "ROTATION", egui::Color32::GRAY, 18.0);
                row(
                    ui,
                    &bracket(rot_str, menu.cursor == 1),
                    egui::Color32::WHITE,
                    24.0,
                );
                ui.add_space(20.0);
                row(
                    ui,
                    &bracket("HI SCORES", menu.cursor == 2),
                    egui::Color32::WHITE,
                    24.0,
                );
                row(
                    ui,
                    &bracket("CONTROLS", menu.cursor == 3),
                    egui::Color32::WHITE,
                    24.0,
                );
                ui.add_space(20.0);
                row(
                    ui,
                    &bracket("START", menu.cursor == 4),
                    egui::Color32::WHITE,
                    24.0,
                );
                ui.add_space(60.0);
                let (label, color) = if pkv.get::<bool>("muted").unwrap_or(false) {
                    ("[M]  MUTED", egui::Color32::from_rgb(204, 102, 102))
                } else {
                    ("[M]  SOUND ON", egui::Color32::GRAY)
                };
                row(ui, label, color, 14.0);
            });
        });

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
