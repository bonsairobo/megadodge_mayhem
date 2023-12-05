mod aabb;
mod ball;
mod boundaries;
mod collision;
mod gym;
mod manager;
mod parameters;
mod player;
mod squad;
mod stats;
mod team;

use ball::{Ball, BallAssets};
use bevy::prelude::*;
use bevy::render::view::NoFrustumCulling;
use bevy_rapier3d::prelude::*;
use boundaries::Boundaries;
use collision::{handle_ball_floor_collisions, handle_ball_player_collisions};
use gym::{Gym, GymAssets, GymParams};
use player::{AvoidPlayers, KnockedOut, Player, PlayerBall, TargetEnemy};
use smooth_bevy_cameras::controllers::orbit::{
    OrbitCameraBundle, OrbitCameraController, OrbitCameraPlugin,
};
use smooth_bevy_cameras::LookTransformPlugin;
use stats::AllStats;
use team::{Team, TeamAssets};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            RapierPhysicsPlugin::<NoUserData>::default(),
            // RapierDebugRenderPlugin::default(),
            LookTransformPlugin,
            OrbitCameraPlugin::default(),
        ))
        .insert_resource(ClearColor(Color::rgb(
            52.0 / 255.0,
            75.0 / 255.0,
            99.0 / 255.0,
        )))
        .add_systems(Startup, (setup, transparency_hack))
        .add_systems(Update, Player::initialize_kinematics)
        .add_systems(
            Update,
            (
                PlayerBall::choose_ball_to_chase,
                // TODO: make these one system?
                PlayerBall::chase_ball.after(PlayerBall::choose_ball_to_chase),
                AvoidPlayers::avoid_other_players,
                TargetEnemy::find_target_enemy,
                Player::throw_ball_at_enemy,
                KnockedOut::update,
                handle_ball_player_collisions,
                handle_ball_floor_collisions,
            )
                .after(Player::initialize_kinematics)
                .before(Player::finalize_kinematics),
        )
        .add_systems(Update, Player::finalize_kinematics)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(Camera3dBundle::default())
        .insert(OrbitCameraBundle::new(
            OrbitCameraController::default(),
            Vec3::new(50.0, 50.0, 0.0),
            Vec3::ZERO,
            Vec3::Y,
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

    let team_assets = TeamAssets::new(&mut meshes, &mut materials);
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

    let team_size = 160;
    Team::spawn(
        &mut commands,
        &team_assets.teams[0],
        0,
        spawn_aabbs[0],
        team_size,
    );
    Team::spawn(
        &mut commands,
        &team_assets.teams[1],
        1,
        spawn_aabbs[1],
        team_size,
    );

    let stats = AllStats {
        squads: vec![default(), default()],
    };

    commands.insert_resource(ball_assets);
    commands.insert_resource(bounds);
    commands.insert_resource(stats);
    commands.insert_resource(team_assets);
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
