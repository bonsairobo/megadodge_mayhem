mod aabb;
mod ball;
mod boundaries;
mod collision;
mod gym;
mod parameters;
mod player;
mod squad;
mod squad_ui;
mod team;

use ball::{Ball, BallAssets};
use bevy::core_pipeline::bloom::BloomSettings;
use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use bevy::window::CursorGrabMode;
use bevy_egui::EguiPlugin;
use bevy_mod_picking::prelude::*;
use bevy_mod_picking::DefaultPickingPlugins;
use bevy_rapier3d::prelude::*;
use boundaries::Boundaries;
use collision::{handle_ball_floor_collisions, handle_ball_player_collisions};
use gym::{Gym, GymAssets, GymParams};
use player::{AvoidPlayers, KnockedOut, Player, PlayerBall, TargetEnemy};
use smooth_bevy_cameras::controllers::orbit::{
    OrbitCameraBundle, OrbitCameraController, OrbitCameraPlugin,
};
use smooth_bevy_cameras::LookTransformPlugin;
use squad::AllSquadAssets;
use squad::SquadAi;
use squad::SquadStates;
use squad::{Squad, SquadBehaviors};
use squad_ui::draw_squad_uis;
use team::AllTeamAssets;

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
        .insert_resource(ClearColor(Color::rgb_u8(52, 75, 99)))
        .insert_resource(RapierBackendSettings {
            require_markers: true,
        })
        .add_systems(Startup, (setup, transparency_hack))
        .add_systems(
            Update,
            (
                grab_mouse,
                draw_squad_uis,
                SquadAi::move_to_requested_positions,
            ),
        )
        .add_systems(Update, Player::initialize_kinematics)
        .add_systems(
            Update,
            (
                PlayerBall::choose_target_ball,
                PlayerBall::pick_up_ball,
                AvoidPlayers::avoid_other_players,
                // TargetEnemy::find_target_enemy,
                // Player::throw_ball_at_enemy,
                Player::follow_leader,
                KnockedOut::update,
                handle_ball_player_collisions,
                handle_ball_floor_collisions,
            )
                .after(Player::initialize_kinematics)
                .before(Player::finalize_kinematics),
        )
        .add_systems(Update, Player::finalize_kinematics)
        .add_systems(PostUpdate, SquadStates::update);
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                // Required for bloom.
                hdr: true,
                ..default()
            },
            ..default()
        })
        .insert((
            BloomSettings::default(),
            OrbitCameraBundle::new(
                OrbitCameraController {
                    mouse_rotate_sensitivity: Vec2::splat(0.3),
                    mouse_translate_sensitivity: Vec2::splat(4.0),
                    mouse_wheel_zoom_sensitivity: 0.2,
                    pixels_per_line: 53.0,
                    smoothing_weight: 0.8,
                    ..default()
                },
                Vec3::new(50.0, 50.0, 0.0),
                Vec3::ZERO,
                Vec3::Y,
            ),
            RapierPickable,
        ));

    let gym_params = GymParams::default();
    let he = gym_params.half_extents();

    // TODO: animated spotlights could look really cool
    let hhe = 0.5 * he;
    let light_positions = [
        Vec3::new(0.0, 5.0, 0.0),
        Vec3::new(-hhe.x, 5.0, -hhe.z),
        Vec3::new(hhe.x, 5.0, -hhe.z),
        Vec3::new(-hhe.x, 5.0, hhe.z),
        Vec3::new(hhe.x, 5.0, hhe.z),
    ];
    for light_position in light_positions {
        commands.spawn(PointLightBundle {
            point_light: PointLight {
                intensity: 1000.0,
                range: 50.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_translation(light_position),
            ..default()
        });
    }

    let gym_assets = GymAssets::new(gym_params, &mut meshes, &mut materials);

    Gym::spawn(&mut commands, &gym_assets);

    let bounds = Boundaries { min: -he, max: he };

    let team_colors = [Color::GREEN, Color::BLUE];
    let squad_teams = [0, 1];
    let squad_colors = [
        team_colors[squad_teams[0] as usize],
        team_colors[squad_teams[1] as usize],
    ];

    let team_assets = AllTeamAssets::new(team_colors, &mut meshes, &mut materials);
    let squad_assets = AllSquadAssets::new(squad_colors, &mut materials);
    let ball_assets = BallAssets::new(&mut meshes, &mut materials);

    let n_balls = 100;
    Ball::spawn_multiple_in_line(
        &mut commands,
        &ball_assets,
        &bounds,
        n_balls,
        [-he.x, 0.0, 0.0].into(),
        [he.x, 0.0, 0.0].into(),
    );

    let spawn_aabbs = gym_params.spawn_aabbs();

    let squad_size = 160;
    let squad_ai_0 = Squad::spawn(
        &mut commands,
        &team_assets.teams[0],
        &squad_assets.squads[0],
        squad_teams[0],
        0,
        spawn_aabbs[0],
        squad_size,
    );
    let squad_ai_1 = Squad::spawn(
        &mut commands,
        &team_assets.teams[1],
        &squad_assets.squads[1],
        1,
        1,
        spawn_aabbs[1],
        squad_size,
    );

    let squad_behaviors = SquadBehaviors::new(vec![squad_ai_0, squad_ai_1]);
    let squad_states = SquadStates::new(vec![squad_size; 2]);

    commands.insert_resource(ball_assets);
    commands.insert_resource(bounds);
    commands.insert_resource(squad_behaviors);
    commands.insert_resource(squad_states);
    commands.insert_resource(team_assets);
    commands.insert_resource(squad_assets);
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
