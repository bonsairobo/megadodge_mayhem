use crate::{
    ball::{Ball, BallAssets},
    collision,
    parameters::{
        AVOID_FACTOR, AVOID_RADIUS, CHASE_FACTOR, CLAIM_RADIUS, PICKUP_RADIUS,
        THROW_COOLDOWN_MILLIS, THROW_START_RADIUS,
    },
    stats::{AllStats, PlayerStats},
    team::Team,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::{
    Collider, ColliderMassProperties, CollisionGroups, QueryFilter, RapierContext, RigidBody,
    Velocity,
};
use std::time::Duration;

#[derive(Component, Default)]
pub struct Player {
    chasing_ball: Option<Entity>,
    throw_cooldown: Timer,
    claimed_ball: bool,
    holding_ball: bool,
    team: u8,
    squad: u8,
}

// TODO: wait time betweeen picking up ball and throwing it

// PERF: we could limit how many spatial queries are done each frame with a
// queue, since every player doesn't **need** to do queries every frame

impl Player {
    pub const DEPTH_LAYER: f32 = 0.0;

    fn new(team: u8, squad: u8) -> Self {
        Self {
            chasing_ball: None,
            throw_cooldown: Timer::new(Duration::from_millis(THROW_COOLDOWN_MILLIS), default()),
            claimed_ball: false,
            holding_ball: false,
            team,
            squad,
        }
    }

    pub fn spawn(
        commands: &mut Commands,
        assets: &PlayerAssets,
        team: u8,
        squad: u8,
        position: Vec2,
    ) {
        commands.spawn((
            Self::new(team, squad),
            Team::new(team),
            SpriteBundle {
                sprite: Sprite {
                    color: assets.color,
                    custom_size: Some(assets.size),
                    ..default()
                },
                transform: Transform::from_translation(position.extend(Self::DEPTH_LAYER)),
                ..default()
            },
            RigidBody::KinematicVelocityBased,
            Velocity::zero(),
            Collider::cuboid(0.5 * assets.size.x, 0.5 * assets.size.y),
            CollisionGroups::new(
                collision::groups::PLAYER,
                collision::groups::QUERY | collision::groups::THROWN_BALL,
            ),
            ColliderMassProperties::Density(1.0),
        ));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        mut commands: Commands,
        time: Res<Time>,
        rapier_context: Res<RapierContext>,
        ball_assets: Res<BallAssets>,
        stats: Res<AllStats>,
        teams: Query<&Team>,
        mut players: Query<(Entity, &Team, &mut Player, &GlobalTransform, &mut Velocity)>,
        player_transforms: Query<&GlobalTransform, With<Player>>,
        mut balls: Query<
            (
                Entity,
                &mut Ball,
                &mut CollisionGroups,
                &GlobalTransform,
                &mut Transform,
            ),
            Without<Player>,
        >,
    ) {
        for (player_entity, player_team, mut player, player_global_tfm, mut player_velocity) in
            players.iter_mut()
        {
            let mut update = PlayerUpdate {
                team: *player_team,
                stats: &stats.squads[player.squad as usize],
                entity: player_entity,
                position: player_global_tfm.translation().xy(),
                player: &mut player,
                // Accumulate velocity vector from multiple competing factors.
                accum_linvel: Vec2::ZERO,
            };

            update.throw_ball_at_enemy(&mut commands, &time, &rapier_context, &ball_assets, &teams);
            update.choose_ball_to_chase(&rapier_context, &mut balls);
            update.chase_ball(&mut commands, &mut balls);
            update.avoid_other_players(&rapier_context, &player_transforms);
            update.set_velocity(&mut player_velocity);
        }
    }

    fn claimant_group_mask(&self) -> u8 {
        debug_assert!(self.team < 8);
        1 << self.team
    }
}

struct PlayerUpdate<'a> {
    team: Team,
    stats: &'a PlayerStats,
    entity: Entity,
    position: Vec2,
    player: &'a mut Player,
    accum_linvel: Vec2,
}

impl<'a> PlayerUpdate<'a> {
    fn set_velocity(&self, velocity: &mut Velocity) {
        assert!(self.accum_linvel.is_finite());
        if self.accum_linvel.length_squared() > 0.0 {
            velocity.linvel = self.accum_linvel;
            velocity.linvel = self.stats.run_speed * velocity.linvel.normalize();
        }
        assert!(velocity.linvel.is_finite(), "{}", self.accum_linvel);
    }

    fn avoid_other_players(
        &mut self,
        rapier_context: &RapierContext,
        player_transforms: &Query<&GlobalTransform, With<Player>>,
    ) {
        let select_all_players = QueryFilter::new().groups(CollisionGroups::new(
            collision::groups::QUERY,
            collision::groups::PLAYER,
        ));
        let mut n_players_nearby = 0;
        let mut sum_nearby_dist = Vec2::ZERO;
        rapier_context.intersections_with_shape(
            self.position,
            0.0,
            &Collider::ball(AVOID_RADIUS),
            select_all_players,
            |other_player_entity| {
                n_players_nearby += 1;
                let Ok(other_player_transform) = player_transforms.get(other_player_entity) else {
                    return true;
                };
                sum_nearby_dist += self.position - other_player_transform.translation().xy();
                true
            },
        );
        if n_players_nearby > 0 {
            self.accum_linvel += AVOID_FACTOR * (sum_nearby_dist / n_players_nearby as f32);
        }
    }

    fn chase_ball(
        &mut self,
        commands: &mut Commands,
        balls: &mut Query<
            (
                Entity,
                &mut Ball,
                &mut CollisionGroups,
                &GlobalTransform,
                &mut Transform,
            ),
            Without<Player>,
        >,
    ) {
        let Some(chasing_ball_entity) = self.player.chasing_ball else {
            return;
        };

        // Check if the player can pick up the ball.
        let Ok((ball_entity, mut ball, mut ball_groups, ball_global_tfm, mut ball_tfm)) =
            balls.get_mut(chasing_ball_entity)
        else {
            return;
        };

        let ball_pos = ball_global_tfm.translation().xy();
        let dist_to_ball = ball_pos.distance(self.position);
        let can_pickup = dist_to_ball <= PICKUP_RADIUS;
        if can_pickup {
            if ball.is_held() {
                // We can't steal the ball.
            } else {
                // Take the ball (regardless of if we claimed it).
                ball.pick_up(&mut ball_tfm, &mut ball_groups);
                self.player.holding_ball = true;
                self.player.throw_cooldown.reset();
                commands.entity(self.entity).push_children(&[ball_entity]);

                // TODO: regroup with squad
            }
            self.player.claimed_ball = false;
            self.player.chasing_ball = None;
        } else {
            // We haven't arrived at the ball yet. Just keep running.
            let run_direction = (ball_pos - self.position).normalize();
            self.accum_linvel += CHASE_FACTOR * self.stats.run_speed * run_direction;

            if !self.player.claimed_ball && dist_to_ball < CLAIM_RADIUS {
                if ball.claim(self.player.claimant_group_mask()) {
                    self.player.claimed_ball = true;
                } else {
                    // Someone already claimed this ball, so search the area for more balls.
                    self.player.chasing_ball = None;
                }
            }
        }
    }

    fn choose_ball_to_chase(
        &mut self,
        rapier_context: &RapierContext,
        balls: &mut Query<
            (
                Entity,
                &mut Ball,
                &mut CollisionGroups,
                &GlobalTransform,
                &mut Transform,
            ),
            Without<Player>,
        >,
    ) {
        if self.player.holding_ball {
            return;
        }

        // Find the nearest unclaimed ball and chase it.
        let can_chase_ball = |ball_entity| {
            if self.player.claimed_ball && self.player.chasing_ball == Some(ball_entity) {
                // It's claimed, but this player claimed it.
                return true;
            }
            let Ok(ball) = balls.get_component::<Ball>(ball_entity) else {
                return false;
            };
            !ball.is_claimed(self.player.claimant_group_mask())
        };
        let select_ground_balls = QueryFilter::new()
            .groups(CollisionGroups::new(
                collision::groups::QUERY,
                collision::groups::GROUND_BALL,
            ))
            .predicate(&can_chase_ball);
        let Some((nearest_ball_entity, _projection)) =
            rapier_context.project_point(self.position, true, select_ground_balls)
        else {
            // No balls!
            return;
        };

        let old_chasing = self.player.chasing_ball;
        self.player.chasing_ball = Some(nearest_ball_entity);
        if self.player.claimed_ball {
            if let Some(old_chasing) = old_chasing {
                // Unclaim the ball
                if let Ok(mut ball) = balls.get_component_mut::<Ball>(old_chasing) {
                    ball.drop_claim(self.player.claimant_group_mask());
                }
            }
        }
    }

    fn throw_ball_at_enemy(
        &mut self,
        commands: &mut Commands,
        time: &Time,
        rapier_context: &RapierContext,
        ball_assets: &BallAssets,
        teams: &Query<&Team>,
    ) {
        if !self.player.holding_ball {
            return;
        }

        if !self.player.throw_cooldown.finished() {
            self.player.throw_cooldown.tick(time.delta());
            return;
        }

        let entity_on_enemy_team = |entity| {
            let Ok(team) = teams.get(entity) else {
                return false;
            };
            team.team() != self.team.team()
        };
        let select_enemy_players = QueryFilter::new()
            .groups(CollisionGroups::new(
                collision::groups::QUERY,
                collision::groups::PLAYER,
            ))
            .predicate(&entity_on_enemy_team);
        // Make sure the player is in a good position to throw at the
        // other team or block incoming balls. There is a very low
        // probability of catching an incoming ball.
        // TODO
        // player_velocity.linvel = Vec2::ZERO;

        // TODO: maybe use claimant bits for this too? It's not
        // necessarily bad for two players to aim at the same target.

        // Identify the closest target.
        let Some((_nearest_player_entity, projection)) =
            rapier_context.project_point(self.position, true, select_enemy_players)
        else {
            // No players!
            return;
        };

        let enemy_pos = projection.point;

        // Check if the enemy is within throwing distance.
        let enemy_dist = enemy_pos.distance(self.position);
        if enemy_dist <= self.stats.throw_distance {
            // Despawn the held ball.
            commands.entity(self.entity).despawn_descendants();
            self.player.holding_ball = false;

            // Spawn a thrown ball.
            let throw_dir = (projection.point - self.position).normalize();
            let throw_velocity = self.stats.throw_speed * throw_dir;
            let throw_start = self.position + throw_dir * THROW_START_RADIUS;
            Ball::spawn_thrown(commands, ball_assets, throw_start, throw_velocity);
        } else {
            // Run towards the enemy.
            let run_direction = (enemy_pos - self.position).normalize();
            self.accum_linvel += CHASE_FACTOR * self.stats.run_speed * run_direction;
        }
    }
}

#[derive(Resource)]
pub struct PlayerAssets {
    pub color: Color,
    pub size: Vec2,
}

impl PlayerAssets {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            size: Vec2::new(8.0, 16.0),
        }
    }
}
