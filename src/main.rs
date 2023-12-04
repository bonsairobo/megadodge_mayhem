mod aabb;
mod ball;
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
use collision::handle_collision_events;
use gym::{Gym, GymAssets, GymParams};
use player::Player;
use stats::AllStats;
use team::{Team, TeamAssets};

// BUG: gravity scaling is way off, balls are falling in slow motion

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
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(500.0, 500.0, 200.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 500_000.0,
            range: 5000.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 200.0, 0.0),
        ..default()
    });

    let gym_params = GymParams::default();
    let gym_assets = GymAssets::new(gym_params, &mut meshes, &mut materials);

    Gym::spawn(&mut commands, &gym_assets);

    let team_assets = TeamAssets::new(&mut meshes, &mut materials);
    let ball_assets = BallAssets::new(&mut meshes, &mut materials);

    let n_balls = 40;
    Ball::spawn_multiple_in_line(
        &mut commands,
        &ball_assets,
        n_balls,
        [-200.0, 0.0, 0.0].into(),
        [200.0, 0.0, 0.0].into(),
    );

    let team0_aabb = Aabb2::new([-200.0, 275.0].into(), [200.0, 325.0].into());
    let team1_aabb = Aabb2::new([-200.0, -325.0].into(), [200.0, -275.0].into());

    let team_size = 80;
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
    commands.insert_resource(stats);
    commands.insert_resource(team_assets);
}
