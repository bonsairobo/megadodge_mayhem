mod avoid_players;
mod knocked_out;

pub use self::avoid_players::*;
pub use self::knocked_out::KnockedOut;

use self::knocked_out::DespawnTimer;
use crate::{
    ball::{BallAssets, ThrownBallBundle},
    boundaries::Boundaries,
    collision,
    parameters::{
        AVOID_FACTOR, CHASE_FACTOR, THROW_COOLDOWN_MILLIS, THROW_LOFT, THROW_OVER_HEAD,
        THROW_TARGET_HEIGHT,
    },
    squad::{Squad, SquadAi, SquadAssets, SquadBehaviors, SquadStates},
    team::{AllTeamAssets, Team, TeamAssets},
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::{
    Collider, ColliderMassProperties, CollisionGroups, LockedAxes, RigidBody, Velocity,
};
use std::time::Duration;

#[derive(Component)]
pub struct Player;

#[derive(Bundle)]
pub struct PlayerBundle {
    pub avoid_players: AvoidPlayers,
    pub ball: PlayerBall,
    pub body: RigidBody,
    pub collider: Collider,
    pub collision_groups: CollisionGroups,
    pub locked_axes: LockedAxes,
    pub mass: ColliderMassProperties,
    pub pbr: PbrBundle,
    pub player: Player,
    pub squad: Squad,
    pub team: Team,
    pub throw_cooldown: ThrowCooldown,
    pub velocity: Velocity,
}

#[derive(Component, Default)]
pub struct PlayerBall {
    pub target_ball: Option<Entity>,
    pub chase_vector: Vec3,
    pub claimed_ball: bool,
    pub holding_ball: bool,
}

impl PlayerBundle {
    pub fn new(
        team_assets: &TeamAssets,
        squad_assets: &SquadAssets,
        team: Team,
        squad: u8,
        position: Vec3,
    ) -> Self {
        Self {
            avoid_players: default(),
            ball: default(),
            body: RigidBody::KinematicVelocityBased,
            collider: Collider::capsule(
                -0.5 * team_assets.capsule_length * Vec3::Y,
                0.5 * team_assets.capsule_length * Vec3::Y,
                team_assets.capsule_radius,
            ),
            collision_groups: Player::in_play_groups(),
            // Prevents unatural amounts of spinning when hit with a ball.
            locked_axes: LockedAxes::ROTATION_LOCKED_Y,
            mass: ColliderMassProperties::Density(1.0),
            player: Player,
            pbr: PbrBundle {
                mesh: team_assets.mesh.clone(),
                material: squad_assets.in_play_material.clone(),
                transform: Transform::from_translation(position),
                ..default()
            },
            squad: Squad::new(squad),
            team,
            throw_cooldown: ThrowCooldown::new(),
            velocity: Velocity::zero(),
        }
    }
}

impl Player {
    fn in_play_groups() -> CollisionGroups {
        CollisionGroups::new(
            collision::groups::PLAYER,
            collision::groups::QUERY
                | collision::groups::THROWN_BALL
                | collision::groups::GROUND_BALL,
        )
    }

    fn out_of_play_groups() -> CollisionGroups {
        CollisionGroups::new(
            collision::groups::PLAYER,
            collision::groups::THROWN_BALL | collision::groups::BOUNDARIES,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn knock_out(
        commands: &mut Commands,
        assets: &AllTeamAssets,
        entity: Entity,
        team: &Team,
        body: &mut RigidBody,
        groups: &mut CollisionGroups,
        material: &mut Handle<StandardMaterial>,
    ) {
        *body = RigidBody::Dynamic;
        *groups = Self::out_of_play_groups();
        let assets = &assets.teams[team.team() as usize];
        *material = assets.out_of_play_material.clone();

        commands
            .entity(entity)
            .try_insert((KnockedOut, DespawnTimer::new()));
    }

    #[allow(clippy::complexity)]
    pub fn initialize_kinematics(
        boundaries: Res<Boundaries>,
        mut players: Query<
            (&GlobalTransform, &mut Transform, &mut Velocity),
            (With<Player>, Without<KnockedOut>),
        >,
    ) {
        for (global_tfm, mut tfm, mut velocity) in &mut players {
            // This gets accumulated from multiple sources over the frame.
            //
            // Systems that are cheap to run will write directly to the
            // velocity, while more expensive systems will write to a buffer to
            // allow for parallelism.
            velocity.linvel = Vec3::ZERO;

            let mut position = global_tfm.translation();
            position = position.clamp(boundaries.min, boundaries.max);
            tfm.translation = position;
        }
    }

    #[allow(clippy::complexity)]
    pub fn finalize_kinematics(
        behaviors: Res<SquadBehaviors>,
        mut players: Query<
            (&Squad, &PlayerBall, &AvoidPlayers, &mut Velocity),
            (With<Player>, Without<KnockedOut>),
        >,
    ) {
        for (squad, ball, avoid_players, mut velocity) in &mut players {
            let stats = &behaviors.squads[squad.squad as usize].stats;

            let mut accum_linvel = velocity.linvel;
            if ball.target_ball.is_some() {
                accum_linvel += CHASE_FACTOR * ball.chase_vector;
            }
            accum_linvel += AVOID_FACTOR * avoid_players.nearby_players_mass;

            // Can't move vertically.
            // TODO: we might want vertical movement for dodges and animations.
            accum_linvel.y = 0.0;

            assert!(accum_linvel.is_finite(), "{}", accum_linvel);
            if let Some(v) = accum_linvel.try_normalize() {
                velocity.linvel = stats.run_speed * v;
            }
            assert!(velocity.linvel.is_finite(), "{}", velocity.linvel);
        }
    }

    #[allow(clippy::complexity)]
    pub fn follow_leader(
        behaviors: Res<SquadBehaviors>,
        states: Res<SquadStates>,
        mut players: Query<
            (&Squad, &GlobalTransform, &mut Velocity),
            (With<Player>, Without<KnockedOut>),
        >,
        leader_transforms: Query<&GlobalTransform, With<SquadAi>>,
    ) {
        for (squad, player_tfm, mut velocity) in &mut players {
            let behavior = &behaviors.squads[squad.squad as usize];
            let Ok(leader_tfm) = leader_transforms.get(behavior.leader) else {
                continue;
            };
            let state = &states.squads[squad.squad as usize];

            let leader_pos = leader_tfm.translation();
            let player_pos = player_tfm.translation();

            // PERF: we could cache parts of this calculation in SquadState
            //
            // It's not good enough to just run to the leader. We want the
            // center of mass to converge on the leader.
            //
            // Each player moves in the direction that the center of mass needs
            // to move towards the target, but weighted by how far they are from
            // the center.
            let player_to_leader = leader_pos - player_pos;
            let com_to_leader = leader_pos - state.center_of_mass;
            let player_to_leader_dist = player_to_leader.length();
            let player_to_cluster_dist = (player_to_leader_dist - state.cluster_radius).max(0.0);
            let player_to_cluster =
                (player_to_cluster_dist / player_to_leader_dist) * player_to_leader;

            // Don't overshoot the cluster.
            let vel_delta = if player_to_cluster_dist > 0.0 || com_to_leader.length_squared() < 1.0
            {
                // If the player is far enough from the cluster, just focus on
                // getting into the cluster. If the squad's center of mass is
                // already close to the leader, then also just focus on being in
                // the cluster.
                player_to_cluster
            } else {
                // Balance trying to be uniform around the center with actually
                // find the correct center.
                //
                // The damping factor avoids a tendency to bunch into the center.
                let dampen_bunching = 0.2;
                com_to_leader + dampen_bunching * player_to_leader
            };

            // This is a tradeoff, because the more damping, the harder it is
            // for clusters to pass through each other.
            let avoid_bunching = 0.5;

            velocity.linvel += CHASE_FACTOR * vel_delta.clamp_length_max(avoid_bunching);
        }
    }

    #[allow(clippy::complexity)]
    pub fn throw_ball_at_enemy(
        mut commands: Commands,
        time: Res<Time>,
        team_assets: Res<AllTeamAssets>,
        ball_assets: Res<BallAssets>,
        behaviors: Res<SquadBehaviors>,
        states: Res<SquadStates>,
        mut players: Query<
            (
                Entity,
                &Team,
                &Squad,
                &GlobalTransform,
                &mut PlayerBall,
                &mut ThrowCooldown,
            ),
            Without<KnockedOut>,
        >,
    ) {
        for (
            player_entity,
            player_team,
            player_squad,
            player_tfm,
            mut player_ball,
            mut throw_cooldown,
        ) in &mut players
        {
            if !player_ball.holding_ball {
                continue;
            }

            if !throw_cooldown.timer.finished() {
                throw_cooldown.timer.tick(time.delta());
                continue;
            }

            let squad_state = &states.squads[player_squad.squad as usize];

            let Some(throw_target) = squad_state.throw_target else {
                // No players!
                continue;
            };
            let player_pos = player_tfm.translation();

            let stats = &behaviors.squads[player_squad.squad as usize].stats;

            // Check if the enemy is within throwing distance.
            let enemy_vector = throw_target - player_pos;
            let enemy_dist = enemy_vector.length();
            if enemy_dist <= stats.throw_distance {
                // Despawn the held ball.
                commands.entity(player_entity).despawn_descendants();
                player_ball.holding_ball = false;

                // Spawn a thrown ball.
                // Start the throw over the player's heads so they don't friendly fire.
                let player_height = team_assets.teams[player_team.team() as usize].size.y;
                let start_y = player_height + THROW_OVER_HEAD;
                let max_y = start_y + THROW_LOFT;
                let end_y = THROW_TARGET_HEIGHT * player_height; // TODO: should look at other team's height
                let throw_v = throw_velocity(enemy_vector.xz(), start_y, max_y, end_y);
                let throw_start = Vec3::new(player_pos.x, start_y, player_pos.z);
                commands.spawn(ThrownBallBundle::new(&ball_assets, throw_start, throw_v));
            } else {
                // Run towards the enemy.
                // TODO: enable but prioritize against following squad AI
                // target_enemy.chase_vector = enemy_direction;
            }
        }
    }
}

#[derive(Component)]
pub struct ThrowCooldown {
    pub timer: Timer,
}

impl ThrowCooldown {
    fn new() -> Self {
        Self {
            timer: Timer::new(Duration::from_millis(THROW_COOLDOWN_MILLIS), default()),
        }
    }
}

fn throw_velocity(lateral_displacement: Vec2, start_y: f32, max_y: f32, end_y: f32) -> Vec3 {
    let g = 9.80665;

    // Constant-acceleration kinematic equation: 0 = (1/2)at^2 + height
    let rise_time = (2.0 * (max_y - start_y) / g).sqrt();
    let fall_time = (2.0 * (max_y - end_y) / g).sqrt();

    let travel_time = rise_time + fall_time;

    // v = d / t
    let lateral_v = lateral_displacement / travel_time;
    let [v_x, v_z] = lateral_v.to_array();

    // Conservation of energy: mgh = (1/2)mv^2
    let v_y = (2.0 * g * (max_y - start_y)).sqrt();

    Vec3::new(v_x, v_y, v_z)
}
