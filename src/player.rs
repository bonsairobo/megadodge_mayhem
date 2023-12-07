mod avoid_players;
mod knocked_out;
mod player_ball;
mod target_enemy;

pub use self::avoid_players::AvoidPlayers;
pub use self::knocked_out::KnockedOut;
pub use self::player_ball::PlayerBall;
pub use self::target_enemy::TargetEnemy;

use self::knocked_out::DespawnTimer;
use crate::{
    ball::{Ball, BallAssets},
    boundaries::Boundaries,
    collision,
    parameters::{AVOID_FACTOR, CHASE_FACTOR, THROW_COOLDOWN_MILLIS, THROW_START_RADIUS},
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
    pub target_enemy: TargetEnemy,
    pub team: Team,
    pub throw_cooldown: ThrowCooldown,
    pub velocity: Velocity,
}

impl PlayerBundle {
    pub fn new(
        team_assets: &TeamAssets,
        squad_assets: &SquadAssets,
        team: u8,
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
            target_enemy: default(),
            team: Team::new(team),
            throw_cooldown: ThrowCooldown::new(),
            velocity: Velocity::zero(),
        }
    }
}

// PERF: we could limit how many spatial queries are done each frame with a
// queue, since every player doesn't **need** to do queries every frame

impl Player {
    fn in_play_groups() -> CollisionGroups {
        CollisionGroups::new(
            collision::groups::PLAYER,
            collision::groups::QUERY | collision::groups::THROWN_BALL,
        )
    }

    fn out_of_play_groups() -> CollisionGroups {
        CollisionGroups::new(
            collision::groups::PLAYER,
            collision::groups::THROWN_BALL | collision::groups::BOUNDARIES,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_out(
        &mut self,
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
            .insert((KnockedOut, DespawnTimer::new()));
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
            (
                &Squad,
                &PlayerBall,
                &TargetEnemy,
                &AvoidPlayers,
                &mut Velocity,
            ),
            (With<Player>, Without<KnockedOut>),
        >,
    ) {
        for (squad, ball, target_enemy, avoid_players, mut velocity) in &mut players {
            let stats = &behaviors.squads[squad.squad as usize].stats;

            let mut accum_linvel = velocity.linvel;
            if ball.target_ball.is_some() {
                accum_linvel += CHASE_FACTOR * ball.chase_vector;
            }
            if target_enemy.target_enemy.is_some() {
                accum_linvel += CHASE_FACTOR * target_enemy.chase_vector;
            }
            accum_linvel += AVOID_FACTOR * avoid_players.nearby_players_mass;

            // Can't move vertically.
            // TODO: we might want vertical movement for dodges and animations.
            accum_linvel.y = 0.0;

            assert!(accum_linvel.is_finite(), "{}", accum_linvel);
            if accum_linvel.length_squared() > 0.0 {
                velocity.linvel = stats.run_speed * accum_linvel.normalize();
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
            let delta = leader_pos - player_pos;
            let dist_to_center = delta.length();

            // HACK: Be more agressive about running into the center of the
            // cluster until most players are inside.
            let delta_to_cluster = if state.cluster_percent() > 80 {
                let dist_to_cluster = (dist_to_center - state.cluster_radius).max(0.0);
                (dist_to_cluster * delta) / dist_to_center
            } else {
                delta
            };
            velocity.linvel += CHASE_FACTOR * delta_to_cluster.clamp_length_max(0.2);
        }
    }

    #[allow(clippy::complexity)]
    pub fn throw_ball_at_enemy(
        mut commands: Commands,
        time: Res<Time>,
        ball_assets: Res<BallAssets>,
        stats: Res<SquadBehaviors>,
        mut players: Query<
            (
                Entity,
                &Squad,
                &GlobalTransform,
                &mut TargetEnemy,
                &mut PlayerBall,
                &mut ThrowCooldown,
            ),
            Without<KnockedOut>,
        >,
        transforms: Query<&GlobalTransform>,
    ) {
        for (
            player_entity,
            player_squad,
            player_tfm,
            mut target_enemy,
            mut player_ball,
            mut throw_cooldown,
        ) in &mut players
        {
            target_enemy.can_target = false;

            if !player_ball.holding_ball {
                continue;
            }

            if !throw_cooldown.timer.finished() {
                throw_cooldown.timer.tick(time.delta());
                // TODO: we could let the player still try to position for a throw
                // while the cooldown is ticking, but we want something smarter than
                // running straight at the enemy
                continue;
            }

            target_enemy.can_target = true;

            // Make sure the player is in a good position to throw at the
            // other team or block incoming balls. There is a very low
            // probability of catching an incoming ball.
            // TODO

            let Some(target_entity) = target_enemy.target_enemy else {
                // No players!
                continue;
            };
            let Ok(enemy_tfm) = transforms.get(target_entity) else {
                continue;
            };
            let player_pos = player_tfm.translation();
            let enemy_pos = enemy_tfm.translation();

            let stats = &stats.squads[player_squad.squad as usize].stats;

            let enemy_direction = (enemy_pos - player_pos).normalize();

            // Check if the enemy is within throwing distance.
            let enemy_dist = enemy_pos.distance(player_pos);
            if enemy_dist <= stats.throw_distance {
                // Despawn the held ball.
                commands.entity(player_entity).despawn_descendants();
                player_ball.holding_ball = false;

                // Spawn a thrown ball.
                let loft = 4.0 * Vec3::Y;
                let throw_velocity = stats.throw_speed * enemy_direction + loft;
                let throw_start = player_pos + enemy_direction * THROW_START_RADIUS;
                Ball::spawn_thrown(&mut commands, &ball_assets, throw_start, throw_velocity);
            } else {
                // Run towards the enemy.
                target_enemy.chase_vector = enemy_direction;
            }
        }
    }
}

#[derive(Component)]
pub struct ThrowCooldown {
    timer: Timer,
}

impl ThrowCooldown {
    fn new() -> Self {
        Self {
            timer: Timer::new(Duration::from_millis(THROW_COOLDOWN_MILLIS), default()),
        }
    }
}
