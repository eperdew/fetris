use crate::menu::main_screen::read_input;
use crate::menu::state::{MenuScreen, MenuState};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

const COL_W: f32 = 200.0;
const ROW_H: f32 = 32.0;

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
                ui.add_space(20.0);

                let gray = egui::Color32::GRAY;
                let light = egui::Color32::LIGHT_GRAY;
                let dark = egui::Color32::DARK_GRAY;

                centered_row(ui, ROW_H, |ui| {
                    centered_cell(ui, COL_W, |ui| {
                        ui.label(egui::RichText::new("KEY").color(gray).size(15.0));
                    });
                    centered_cell(ui, COL_W, |ui| {
                        ui.label(egui::RichText::new("ACTION").color(gray).size(15.0));
                    });
                });
                ui.add(egui::Separator::default().horizontal().spacing(8.0));
                ui.add_space(4.0);

                let rows: &[(&str, &str)] = &[
                    ("Left / H", "Move left"),
                    ("Right / L", "Move right"),
                    ("Down / J", "Soft drop"),
                    ("Space", "Sonic drop"),
                    ("X", "Rotate CW"),
                    ("Z", "Rotate CCW"),
                    ("Backspace", "Back / quit"),
                    ("M", "Toggle mute"),
                ];
                for (k, a) in rows {
                    centered_row(ui, ROW_H, |ui| {
                        centered_cell(ui, COL_W, |ui| {
                            ui.label(egui::RichText::new(*k).color(light).size(20.0));
                        });
                        centered_cell(ui, COL_W, |ui| {
                            ui.label(egui::RichText::new(*a).color(light).size(20.0));
                        });
                    });
                }

                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("BKSP to go back")
                        .color(dark)
                        .size(14.0),
                );
            });
        });
}

fn centered_row(ui: &mut egui::Ui, height: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        add_contents(ui);
    });
    let _ = height;
}

fn centered_cell(ui: &mut egui::Ui, width: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::Vec2::new(width, 28.0),
        egui::Layout::top_down(egui::Align::Center),
        add_contents,
    );
}
