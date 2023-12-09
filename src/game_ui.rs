use crate::{
    restart_game::RestartGame,
    settings::{GameSettings, SaveSettings},
};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use smooth_bevy_cameras::controllers::orbit::OrbitCameraController;

#[derive(Resource)]
pub struct GameUi {
    pub show: bool,
}

impl Default for GameUi {
    fn default() -> Self {
        Self { show: true }
    }
}

impl GameUi {
    pub fn toggle(mut game_ui: ResMut<Self>, key: Res<Input<KeyCode>>) {
        if key.just_pressed(KeyCode::G) {
            game_ui.show ^= true;
        }
    }

    pub fn update(
        mut commands: Commands,
        mut save_events: EventWriter<SaveSettings>,
        mut game_ui: ResMut<Self>,
        mut settings: ResMut<GameSettings>,
        mut contexts: EguiContexts,
        mut cameras: Query<&mut OrbitCameraController>,
    ) {
        if !game_ui.show {
            return;
        }

        let mut cam_changed = false;
        egui::Window::new("Mega Dodge Mayhem")
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .show(contexts.ctx_mut(), |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("Restart Game").clicked() {
                        game_ui.show = !settings.hide_menu_when_game_starts;
                        commands.add(RestartGame);
                    }
                    if ui.button("Save Settings").clicked() {
                        save_events.send(SaveSettings);
                    }
                    ui.checkbox(
                        &mut settings.hide_menu_when_game_starts,
                        "Hide this menu when game starts",
                    );
                });
                ui.separator();

                ui.vertical_centered(|ui| {
                    ui.heading("Game Settings");
                });
                ui.separator();
                ui.add(
                    egui::Slider::new(&mut settings.next_game.players_per_squad, 1..=5000)
                        .text("Players Per Squad"),
                );
                ui.add(
                    egui::Slider::new(&mut settings.next_game.squads_per_team, 1..=5)
                        .text("Squads Per Team"),
                );
                ui.add(egui::Slider::new(&mut settings.next_game.n_balls, 0..=2000).text("Balls"));
                ui.separator();

                ui.collapsing("Controls", |ui| {
                    egui::Grid::new("controls")
                        .striped(true)
                        .min_col_width(100.0)
                        .show(ui, |ui| {
                            ui.label("G");
                            ui.label("toggle this window");
                            ui.end_row();

                            ui.label("Left Click");
                            ui.label("select squad and place leader tokens");
                            ui.end_row();

                            ui.label("Ctrl + Mouse");
                            ui.label("rotate camera");
                            ui.end_row();

                            ui.vertical(|ui| {
                                ui.label("Right Click Drag");
                                ui.label("(or Alt + Mouse)");
                            });
                            ui.label("translate camera");
                            ui.end_row();

                            ui.label("Mouse Wheel");
                            ui.label("zoom");
                            ui.end_row();

                            ui.label("D");
                            ui.label("toggle debug UI");
                            ui.end_row();
                        });
                });
                ui.separator();

                ui.collapsing("Camera Settings", |ui| {
                    cam_changed |= ui
                        .add(
                            egui::Slider::new(&mut settings.rotate_sensitivity, 0.01..=0.5)
                                .text("Rotate Sensitivity"),
                        )
                        .changed();
                    cam_changed |= ui
                        .add(
                            egui::Slider::new(&mut settings.translate_sensitivity, 0.01..=5.0)
                                .text("Translate Sensitivity"),
                        )
                        .changed();
                    cam_changed |= ui
                        .add(
                            egui::Slider::new(&mut settings.zoom_sensitivity, 0.01..=1.0)
                                .text("Zoom Sensitivity"),
                        )
                        .changed();
                });
            });

        if cam_changed {
            if let Ok(mut cam) = cameras.get_single_mut() {
                settings.apply_to_camera(&mut cam);
            };
        }
    }
}
