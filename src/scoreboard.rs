use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

#[derive(Resource)]
pub struct ScoreBoard {
    pub team_scores: Vec<u32>,
}

impl ScoreBoard {
    pub fn new(n_teams: u8) -> Self {
        Self {
            team_scores: vec![0; n_teams as usize],
        }
    }

    pub fn draw(scoreboard: Res<Self>, mut contexts: EguiContexts) {
        egui::Window::new("Your Team")
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(0.0, 0.0))
            .interactable(false)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .show(contexts.ctx_mut(), |ui| {
                ui.label(format!("{}", scoreboard.team_scores[0]));
            });
        egui::Window::new("The Bad Guys")
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(0.0, 0.0))
            .interactable(false)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .show(contexts.ctx_mut(), |ui| {
                ui.label(format!("{}", scoreboard.team_scores[1]));
            });
    }
}
