use crate::{
    ball::{Ball, BallAssets},
    boundaries::Boundaries,
    gym::{Gym, GymAssets, GymParams},
    settings::{GameSettings, NextGameConfig},
    squad::{AllSquadAssets, Squad, SquadBehaviors, SquadStates},
    team::AllTeamAssets,
};
use bevy::{
    core_pipeline::bloom::BloomSettings,
    ecs::system::{Command, RunSystemOnce},
    prelude::*,
};
use bevy_mod_picking::prelude::RapierPickable;
use smooth_bevy_cameras::controllers::orbit::OrbitCameraBundle;

pub struct RestartGame;

impl Command for RestartGame {
    fn apply(self, world: &mut bevy::prelude::World) {
        world.run_system_once(destroy_scene);
        world.run_system_once(start_game);
    }
}

fn destroy_scene(mut commands: Commands, all_scene: Query<Entity, With<GlobalTransform>>) {
    for entity in &all_scene {
        commands.entity(entity).despawn();
    }
}

pub fn start_game(
    mut commands: Commands,
    settings: Res<GameSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let gym_params = GymParams::default();
    let he = gym_params.half_extents();
    let gym_assets = GymAssets::new(gym_params, &mut meshes, &mut materials);
    Gym::spawn(&mut commands, &gym_assets);
    let bounds = Boundaries { min: -he, max: he };
    let player_spawn_aabbs = gym_params.player_spawn_aabbs(16.0);
    let ball_spawn_aabb = gym_params.ball_spawn_aabb(4.0);
    let occupancy = gym_params.occupancy_grid();

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
                settings.make_camera(),
                Vec3::new(100.0, 100.0, 0.0),
                Vec3::ZERO,
                Vec3::Y,
            ),
            RapierPickable,
        ));

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
                intensity: 2000.0,
                range: 50.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_translation(light_position),
            ..default()
        });
    }

    let NextGameConfig {
        squads_per_team,
        players_per_squad,
        n_balls,
    } = settings.next_game;

    let ball_assets = BallAssets::new(&mut meshes, &mut materials);
    Ball::spawn_multiple_in_aabb(
        &mut commands,
        &ball_assets,
        &bounds,
        ball_spawn_aabb,
        n_balls,
    );

    let team_colors = [Color::GREEN, Color::BLUE];
    let squad_teams: Vec<_> = std::iter::repeat(0u8)
        .take(usize::from(squads_per_team))
        .chain(std::iter::repeat(1).take(usize::from(squads_per_team)))
        .collect();
    let n_squads = squad_teams.len();

    let squad_colors: Vec<_> = squad_teams
        .iter()
        .map(|&t| team_colors[t as usize])
        .collect();
    let team_assets = AllTeamAssets::new(team_colors, &mut meshes, &mut materials);
    let squad_assets = AllSquadAssets::new(squad_colors, &mut materials);

    let mut squad_ai_entities = Vec::new();
    Squad::spawn_in_line(
        &mut commands,
        &team_assets.teams[0],
        &squad_assets,
        0,
        0..squads_per_team,
        player_spawn_aabbs[0],
        players_per_squad,
        &mut squad_ai_entities,
    );
    Squad::spawn_in_line(
        &mut commands,
        &team_assets.teams[1],
        &squad_assets,
        1,
        squads_per_team..2 * squads_per_team,
        player_spawn_aabbs[1],
        players_per_squad,
        &mut squad_ai_entities,
    );

    let squad_behaviors = SquadBehaviors::new(squad_ai_entities);
    let squad_states = SquadStates::new(vec![players_per_squad; n_squads]);

    commands.insert_resource(ball_assets);
    commands.insert_resource(bounds);
    commands.insert_resource(occupancy);
    commands.insert_resource(squad_behaviors);
    commands.insert_resource(squad_states);
    commands.insert_resource(team_assets);
    commands.insert_resource(squad_assets);
}
