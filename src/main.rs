mod aabb;
mod ball;
mod collision;
mod cooldown;
mod manager;
mod parameters;
mod player;
mod squad;
mod stats;
mod team;

use aabb::Aabb2;
use ball::{Ball, BallAssets};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use collision::handle_collision_events;
use player::Player;
use static_assertions::const_assert;
use stats::AllStats;
use team::{Team, TeamAssets};

// Balls are drawn on top of players.
const_assert!(Player::DEPTH_LAYER < Ball::DEPTH_LAYER);

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
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
        ))
        .insert_resource(RapierConfiguration {
            gravity: Vec2::ZERO,
            ..default()
        })
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
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let team_assets = TeamAssets::default();
    let ball_assets = BallAssets::new(&mut meshes, &mut materials);

    let n_balls = 50;
    Ball::spawn_multiple_in_line(
        &mut commands,
        &ball_assets,
        n_balls,
        [-200.0, 0.0].into(),
        [200.0, 0.0].into(),
    );

    let team0_aabb = Aabb2::new([-200.0, 275.0].into(), [200.0, 325.0].into());
    let team1_aabb = Aabb2::new([-200.0, -325.0].into(), [200.0, -275.0].into());

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
    commands.insert_resource(stats);
    commands.insert_resource(team_assets);
}
