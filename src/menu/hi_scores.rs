use crate::menu::main_screen::read_input;
use crate::menu::state::{MenuScreen, MenuState};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

pub fn hi_scores_system(
    mut contexts: EguiContexts,
    mut menu: ResMut<MenuState>,
    keys: Res<ButtonInput<KeyCode>>,
    hi_scores: Res<crate::stub_storage::HiScoresRes>,
) {
    if menu.screen != MenuScreen::HiScores {
        return;
    }
    let input = read_input(&keys);
    if input.back {
        menu.screen = MenuScreen::Main;
        return;
    }
    if input.left {
        menu.hi_scores_tab = menu.hi_scores_tab.saturating_sub(1);
    }
    if input.right {
        menu.hi_scores_tab = (menu.hi_scores_tab + 1).min(3);
    }

    let tab_names = ["MASTER / ARS", "MASTER / SRS", "20G / ARS", "20G / SRS"];
    let tab = menu.hi_scores_tab;
    let entries = &hi_scores.0[tab];

    let ctx = contexts.ctx_mut().expect("egui context");
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(egui::Color32::from_rgb(10, 10, 18)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(120.0);
                ui.label(
                    egui::RichText::new(format!("< {} >", tab_names[tab]))
                        .color(egui::Color32::WHITE)
                        .size(26.0),
                );
                ui.add_space(40.0);
                egui::Grid::new("hi_scores_grid")
                    .num_columns(3)
                    .spacing([60.0, 12.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("#")
                                .color(egui::Color32::GRAY)
                                .size(15.0),
                        );
                        ui.label(
                            egui::RichText::new("GRADE")
                                .color(egui::Color32::GRAY)
                                .size(15.0),
                        );
                        ui.label(
                            egui::RichText::new("TIME")
                                .color(egui::Color32::GRAY)
                                .size(15.0),
                        );
                        ui.end_row();
                        for i in 0..5 {
                            let color = if i == 0 {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::LIGHT_GRAY
                            };
                            ui.label(
                                egui::RichText::new(format!("{}", i + 1))
                                    .color(color)
                                    .size(20.0),
                            );
                            if let Some(e) = entries.get(i) {
                                ui.label(
                                    egui::RichText::new(format!("{}", e.grade))
                                        .color(color)
                                        .size(20.0),
                                );
                                ui.label(
                                    egui::RichText::new(crate::render::hud::format_time(e.ticks))
                                        .color(color)
                                        .size(20.0),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("---")
                                        .color(egui::Color32::DARK_GRAY)
                                        .size(20.0),
                                );
                                ui.label(
                                    egui::RichText::new("---")
                                        .color(egui::Color32::DARK_GRAY)
                                        .size(20.0),
                                );
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new("BKSP to go back")
                        .color(egui::Color32::GRAY)
                        .size(14.0),
                );
            });
        });
}
