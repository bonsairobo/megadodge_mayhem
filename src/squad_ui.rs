use crate::{
    squad::{Squad, SquadAi, SquadStates},
    team::Team,
};
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Color32},
    EguiContexts,
};

#[derive(Default, Resource)]
pub struct SquadUi {
    pub show_debug: bool,
}

impl SquadUi {
    pub fn toggle(mut ui: ResMut<Self>, key: Res<Input<KeyCode>>) {
        if key.just_pressed(KeyCode::D) {
            ui.show_debug ^= true;
        }
    }

    pub fn draw(
        ui: Res<Self>,
        mut gizmos: Gizmos,
        mut giz_config: ResMut<GizmoConfig>,
        mut egui: EguiContexts,
        squad_states: Res<SquadStates>,
        cameras: Query<(&Camera, &GlobalTransform)>,
        squad_ais: Query<(&Team, &Squad, &GlobalTransform), With<SquadAi>>,
    ) {
        let ctx = egui.ctx_mut();
        let (camera, camera_tfm) = cameras.single();
        let Some(viewport_rect) = camera.logical_viewport_rect() else {
            return;
        };

        giz_config.depth_bias = -1.0;
        giz_config.line_width = 3.0;
        // giz_config.line_perspective = true;

        let transparent_white = Color32::from_rgba_unmultiplied(255, 255, 255, 64);
        let stroke = egui::Stroke::new(3.0, transparent_white);

        for (team, squad, tfm) in &squad_ais {
            let to_egui_pos = |v: Vec2| egui::pos2(v.x, v.y);
            let dbg_painter = ctx.layer_painter(egui::LayerId::background());

            let ai_pos = tfm.translation();
            let Some(ai_viewport_pos) = camera.world_to_viewport(camera_tfm, ai_pos) else {
                continue;
            };
            let ai_window_pos = ai_viewport_pos + viewport_rect.min;

            let state = &squad_states.squads[squad.squad as usize];
            if state.num_players == 0 {
                continue;
            }

            if team.is_human() {
                gizmos.line(state.center_of_mass, tfm.translation(), Color::WHITE);
                dbg_painter.circle(
                    to_egui_pos(ai_window_pos),
                    10.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 32),
                    stroke,
                );
            }

            if !ui.show_debug {
                continue;
            }

            gizmos.circle(
                state.center_of_mass,
                Vec3::Y,
                state.cluster_radius,
                Color::rgb_u8(255, 255, 255),
            );

            let text = format!(
                "SQUAD {}\nplayers: {}\nballs: {}% ({})\ncluster: {}%",
                squad.squad,
                state.num_players,
                state.ball_percent(),
                state.num_holding_balls,
                state.cluster_percent()
            );
            let alignment = egui::Align2::LEFT_TOP;
            dbg_painter.debug_text(
                (to_egui_pos(ai_window_pos).to_vec2()
                    - alignment.to_sign() * egui::vec2(20.0, 20.0))
                .to_pos2(),
                alignment,
                egui::Color32::WHITE,
                text,
            );
        }
    }
}
