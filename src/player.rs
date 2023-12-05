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
    squad::Squad,
    stats::AllStats,
    team::{Team, TeamAssets},
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::{
    Collider, ColliderMassProperties, CollisionGroups, LockedAxes, RigidBody, Velocity,
};
use std::time::Duration;

#[derive(Component)]
pub struct Player;

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
        assets: &TeamAssets,
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

    pub fn spawn(
        commands: &mut Commands,
        assets: &PlayerAssets,
        team: u8,
        squad: u8,
        mut position: Vec3,
    ) {
        position.y = 0.5 * assets.size.y;
        commands.spawn((
            Self,
            Team::new(team),
            Squad { squad },
            ThrowCooldown::new(),
            PlayerBall::default(),
            TargetEnemy::default(),
            AvoidPlayers::default(),
            PbrBundle {
                mesh: assets.mesh.clone(),
                material: assets.in_play_material.clone(),
                transform: Transform::from_translation(position),
                ..default()
            },
            RigidBody::KinematicVelocityBased,
            Velocity::zero(),
            Collider::capsule(
                -0.5 * assets.capsule_length * Vec3::Y,
                0.5 * assets.capsule_length * Vec3::Y,
                assets.capsule_radius,
            ),
            Self::in_play_groups(),
            ColliderMassProperties::Density(1.0),
            // Prevents unatural amounts of spinning when hit with a ball.
            LockedAxes::ROTATION_LOCKED_Y,
        ));
    }

    #[allow(clippy::complexity)]
    pub fn initialize_kinematics(
        boundaries: Res<Boundaries>,
        mut players: Query<(&GlobalTransform, &mut Transform), (With<Player>, Without<KnockedOut>)>,
    ) {
        for (global_tfm, mut tfm) in &mut players {
            let mut position = global_tfm.translation();
            position = position.clamp(boundaries.min, boundaries.max);
            tfm.translation = position;
        }
    }

    #[allow(clippy::complexity)]
    pub fn finalize_kinematics(
        stats: Res<AllStats>,
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
            let stats = &stats.squads[squad.squad as usize];

            let mut accum_linvel = Vec3::ZERO;
            if ball.chasing_ball.is_some() {
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
    pub fn throw_ball_at_enemy(
        mut commands: Commands,
        time: Res<Time>,
        ball_assets: Res<BallAssets>,
        stats: Res<AllStats>,
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

            let stats = &stats.squads[player_squad.squad as usize];

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

pub struct PlayerAssets {
    pub color: Color,
    pub size: Vec3,
    pub capsule_radius: f32,
    pub capsule_length: f32,
    pub in_play_material: Handle<StandardMaterial>,
    pub out_of_play_material: Handle<StandardMaterial>,
    pub mesh: Handle<Mesh>,
}

impl PlayerAssets {
    pub fn new(
        color: Color,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        // 1.8 meters tall.
        let height = 1.8;
        let capsule_radius = 0.18;
        let diam = 2.0 * capsule_radius;
        let capsule_length = height - diam;
        let size = Vec3::new(diam, height, diam);
        Self {
            color,
            size,
            capsule_radius,
            capsule_length,
            mesh: meshes.add(
                shape::Capsule {
                    radius: capsule_radius,
                    depth: capsule_length,
                    ..default()
                }
                .try_into()
                .unwrap(),
            ),
            in_play_material: materials.add(color.into()),
            out_of_play_material: materials.add(color.with_a(0.2).into()),
        }
    }
}
