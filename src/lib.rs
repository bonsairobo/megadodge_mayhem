mod ball;
mod boundaries;
mod collision;
mod game_ui;
mod geometry;
mod grid2;
mod gym;
mod occupancy_grid;
mod opponent_ai;
mod parameters;
mod player;
mod restart_game;
mod settings;
mod squad;
mod squad_ui;
mod team;

use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use bevy::window::CursorGrabMode;
use bevy_egui::egui;
use bevy_egui::egui::Color32;
use bevy_egui::{EguiContexts, EguiPlugin};
use bevy_mod_picking::prelude::*;
use bevy_mod_picking::DefaultPickingPlugins;
use bevy_pkv::PkvStore;
use bevy_rapier3d::prelude::*;
use collision::{handle_ball_floor_collisions, handle_ball_player_collisions};
use game_ui::GameUi;
use occupancy_grid::OccupancyGrid;
use opponent_ai::control_bot_team;
use player::{AvoidPlayers, KnockedOut, Player};
use restart_game::start_game;
use settings::{GameSettings, SaveSettings};
use smooth_bevy_cameras::controllers::orbit::OrbitCameraPlugin;
use smooth_bevy_cameras::LookTransformPlugin;
use squad::{SquadAi, SquadStates};
use squad_ui::SquadUi;

// IDEAS
// - draw line from center of mass to leader token
// - add point-buy for squad statistics
// - make the throw loft adjustable, using a gizmo to show the arc
// - make players holding balls run to the "front" of their cluster
// - let players holding balls run closer to the current target enemy if they're
//   out of range
// - let players leave their cluster to quickly grab a nearby ball
// - let players block incoming thrown balls if they are holding a ball
// - make the cluster shape adjustable
// - let players dodge out of the way of thrown balls

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            EguiPlugin,
            DefaultPickingPlugins,
            RapierPhysicsPlugin::<NoUserData>::default(),
            // RapierDebugRenderPlugin::default(),
            LookTransformPlugin,
            OrbitCameraPlugin::default(),
        ))
        .add_event::<SaveSettings>()
        .insert_resource(PkvStore::new("bonsairobo", "MegaDodgeMayhem"))
        .insert_resource(ClearColor(Color::rgb_u8(52, 75, 99)))
        .insert_resource(RapierBackendSettings {
            require_markers: true,
        })
        .init_resource::<GameSettings>()
        .init_resource::<GameUi>()
        .init_resource::<SquadUi>()
        .add_systems(
            Startup,
            (
                GameSettings::load,
                start_game.after(GameSettings::load),
                transparency_hack,
                configure_egui_visuals,
            ),
        )
        .add_systems(PreUpdate, emulate_right_click_with_alt)
        // Needs to be PreUpdate so AI can use correct SquadState values
        .add_systems(PreUpdate, SquadStates::update)
        .add_systems(
            Update,
            (
                grab_mouse,
                GameUi::update,
                GameUi::toggle,
                SquadUi::toggle,
                SquadUi::draw,
                OccupancyGrid::update,
                SquadAi::move_to_requested_positions,
                SquadAi::find_target_enemy,
                control_bot_team,
            ),
        )
        .add_systems(Update, Player::initialize_kinematics)
        .add_systems(
            Update,
            (
                AvoidPlayers::avoid_other_players,
                Player::throw_ball_at_enemy,
                Player::follow_leader,
                KnockedOut::update,
                handle_ball_player_collisions,
                handle_ball_floor_collisions,
            )
                .after(Player::initialize_kinematics)
                .before(Player::finalize_kinematics),
        )
        .add_systems(Update, Player::finalize_kinematics)
        .add_systems(Last, GameSettings::save_on_exit_or_request);
    }
}

// Mostly for laptops without good right click and drag support.
fn emulate_right_click_with_alt(keys: Res<Input<KeyCode>>, mut mouse: ResMut<Input<MouseButton>>) {
    if keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight) {
        mouse.press(MouseButton::Right);
    }
    if keys.just_released(KeyCode::AltLeft) || keys.just_released(KeyCode::AltRight) {
        mouse.release(MouseButton::Right);
    }
}

fn grab_mouse(
    mut windows: Query<&mut Window>,
    mouse: Res<Input<MouseButton>>,
    key: Res<Input<KeyCode>>,
) {
    let mut window = windows.single_mut();

    if mouse.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }

    if key.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
    }
}

fn configure_egui_visuals(mut contexts: EguiContexts) {
    contexts.ctx_mut().set_visuals(egui::Visuals {
        window_rounding: 0.0.into(),
        faint_bg_color: Color32::from_gray(50),
        ..Default::default()
    });
}

// HACK: front-load a stutter that occurs the first time a transparent material
// is rendered
fn transparency_hack(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let transparent_material = materials.add(Color::GRAY.with_a(0.01).into());
    commands
        .spawn(PbrBundle {
            material: transparent_material,
            // Keep it out of site.
            transform: Transform::from_xyz(0.0, -f32::INFINITY, 0.0),
            mesh: meshes.add(
                shape::Plane {
                    size: 0.0,
                    subdivisions: 0,
                }
                .try_into()
                .unwrap(),
            ),
            ..default()
        })
        // Needs to actually get drawn.
        .insert(NoFrustumCulling);
}
