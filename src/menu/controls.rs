use crate::menu::main_screen::read_input;
use crate::menu::state::{MenuScreen, MenuState};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

pub fn controls_system(
    mut contexts: EguiContexts,
    mut menu: ResMut<MenuState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if menu.screen != MenuScreen::Controls {
        return;
    }
    if read_input(&keys).back {
        menu.screen = MenuScreen::Main;
        return;
    }

    let ctx = contexts.ctx_mut().expect("egui context");
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(egui::Color32::from_rgb(10, 10, 18)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(100.0);
                ui.label(
                    egui::RichText::new("CONTROLS")
                        .color(egui::Color32::WHITE)
                        .size(26.0),
                );
                ui.add_space(40.0);
                let rows: &[(&str, &str)] = &[
                    ("Left / H", "Move left"),
                    ("Right / L", "Move right"),
                    ("Down / J", "Soft drop"),
                    ("Space", "Sonic drop"),
                    ("X", "Rotate CW"),
                    ("Z", "Rotate CCW"),
                    ("Backspace", "Back / quit"),
                ];
                egui::Grid::new("controls_grid")
                    .num_columns(2)
                    .spacing([40.0, 12.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("KEY")
                                .color(egui::Color32::GRAY)
                                .size(15.0),
                        );
                        ui.label(
                            egui::RichText::new("ACTION")
                                .color(egui::Color32::GRAY)
                                .size(15.0),
                        );
                        ui.end_row();
                        for (k, a) in rows {
                            ui.label(
                                egui::RichText::new(*k)
                                    .color(egui::Color32::LIGHT_GRAY)
                                    .size(20.0),
                            );
                            ui.label(
                                egui::RichText::new(*a)
                                    .color(egui::Color32::LIGHT_GRAY)
                                    .size(20.0),
                            );
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
