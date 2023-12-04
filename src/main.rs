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

use aabb::Aabb2;
use ball::{Ball, BallAssets};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use boundaries::Boundaries;
use collision::handle_collision_events;
use gym::{Gym, GymAssets, GymParams};
use player::Player;
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
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                Player::update,
                Ball::handle_cooldown,
                handle_collision_events,
            ),
        )
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
            Vec3::new(25.0, 25.0, 0.0),
            Vec3::ZERO,
            Vec3::Y,
        ));

    // TODO: more lights
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1000.0,
            range: 50.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 5.0, 0.0),
        ..default()
    });

    let gym_params = GymParams::default();
    let gym_assets = GymAssets::new(gym_params, &mut meshes, &mut materials);

    Gym::spawn(&mut commands, &gym_assets);

    let he = gym_params.half_extents();
    let boundaries = Boundaries { min: -he, max: he };

    let team_assets = TeamAssets::new(&mut meshes, &mut materials);
    let ball_assets = BallAssets::new(&mut meshes, &mut materials);

    let n_balls = 20;
    Ball::spawn_multiple_in_line(
        &mut commands,
        &ball_assets,
        n_balls,
        [-9.0, 0.0, 0.0].into(),
        [9.0, 0.0, 0.0].into(),
    );

    let team0_aabb = Aabb2::new([-10.0, 15.0 - 1.0].into(), [10.0, 15.0 + 1.0].into());
    let team1_aabb = Aabb2::new([-10.0, -15.0 - 1.0].into(), [10.0, -15.0 + 1.0].into());

    let team_size = 40;
    Team::spawn(
        &mut commands,
        &team_assets.teams[0],
        0,
        team0_aabb,
        team_size,
    );
    Team::spawn(
        &mut commands,
        &team_assets.teams[1],
        1,
        team1_aabb,
        team_size,
    );

    let stats = AllStats {
        squads: vec![default(), default()],
    };

    commands.insert_resource(ball_assets);
    commands.insert_resource(boundaries);
    commands.insert_resource(stats);
    commands.insert_resource(team_assets);
}
