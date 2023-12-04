use crate::{
    ball::{Ball, BallAssets},
    collision,
    parameters::{
        AVOID_FACTOR, AVOID_RADIUS, CHASE_FACTOR, CLAIM_RADIUS, PICKUP_RADIUS, THROW_START_RADIUS,
    },
    stats::AllStats,
    team::Team,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::{
    Collider, ColliderMassProperties, CollisionGroups, QueryFilter, RapierContext, RigidBody,
    Velocity,
};

#[derive(Component, Default)]
pub struct Player {
    chasing_ball: Option<Entity>,
    claimed_ball: bool,
    holding_ball: bool,
    team: u8,
    squad: u8,
}

// TODO: wait time betweeen picking up ball and throwing it

impl Player {
    pub const DEPTH_LAYER: f32 = 1.0;

    fn new(team: u8, squad: u8) -> Self {
        Self {
            chasing_ball: None,
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
            let squad_stats = &stats.squads[player.squad as usize];

            let player_pos = player_global_tfm.translation().xy();

            // Accumulate velocity vector from multiple competing factors.
            let mut accum_linvel = Vec2::ZERO;

            if player.holding_ball {
                // Make sure the player is in a good position to throw at the
                // other team or block incoming balls. There is a very low
                // probability of catching an incoming ball.
                // TODO
                // player_velocity.linvel = Vec2::ZERO;

                // TODO: maybe use claimant bits for this too? It's not
                // necessarily bad for two players to aim at the same target.

                // Identify the closest target.
                let player_team = player_team.team();
                let entity_on_enemy_team = |entity| {
                    let Ok(team) = teams.get(entity) else {
                        return false;
                    };
                    team.team() != player_team
                };
                let select_enemy_players = QueryFilter::new()
                    .groups(CollisionGroups::new(
                        collision::groups::QUERY,
                        collision::groups::PLAYER,
                    ))
                    .predicate(&entity_on_enemy_team);
                if let Some((_nearest_player_entity, projection)) =
                    rapier_context.project_point(player_pos, true, select_enemy_players)
                {
                    let enemy_pos = projection.point;

                    // Check if the enemy is within throwing distance.
                    let enemy_dist = enemy_pos.distance(player_pos);
                    if enemy_dist <= squad_stats.throw_distance {
                        // Despawn the held ball.
                        commands.entity(player_entity).despawn_descendants();
                        player.holding_ball = false;

                        // Spawn a thrown ball.
                        let throw_dir = (projection.point - player_pos).normalize();
                        let throw_velocity = squad_stats.throw_speed * throw_dir;
                        let throw_start = player_pos + throw_dir * THROW_START_RADIUS;
                        Ball::spawn_thrown(
                            &mut commands,
                            &ball_assets,
                            throw_start,
                            throw_velocity,
                        );
                    } else {
                        // Run towards the enemy.
                        let run_direction = (enemy_pos - player_pos).normalize();
                        accum_linvel += CHASE_FACTOR * squad_stats.run_speed * run_direction;
                    }
                } else {
                    // No players!
                }
            }

            if !player.holding_ball {
                // Find the nearest unclaimed ball and chase it.
                let can_chase_ball = |ball_entity| {
                    if player.claimed_ball && player.chasing_ball == Some(ball_entity) {
                        // It's claimed, but this player claimed it.
                        return true;
                    }
                    let Ok(ball) = balls.get_component::<Ball>(ball_entity) else {
                        return false;
                    };
                    !ball.is_claimed(player.claimant_group_mask())
                };
                let select_ground_balls = QueryFilter::new()
                    .groups(CollisionGroups::new(
                        collision::groups::QUERY,
                        collision::groups::GROUND_BALL,
                    ))
                    .predicate(&can_chase_ball);
                if let Some((nearest_ball_entity, _projection)) =
                    rapier_context.project_point(player_pos, true, select_ground_balls)
                {
                    let old_chasing = player.chasing_ball;
                    player.chasing_ball = Some(nearest_ball_entity);
                    if player.claimed_ball {
                        if let Some(old_chasing) = old_chasing {
                            // Unclaim the ball
                            if let Ok(mut ball) = balls.get_component_mut::<Ball>(old_chasing) {
                                ball.drop_claim(player.claimant_group_mask());
                            }
                        }
                    }
                } else {
                    // No balls!
                }
            }

            if let Some(chasing_ball_entity) = player.chasing_ball {
                // Check if the player can pick up the ball.
                if let Ok((ball_entity, mut ball, mut ball_groups, ball_global_tfm, mut ball_tfm)) =
                    balls.get_mut(chasing_ball_entity)
                {
                    let ball_pos = ball_global_tfm.translation().xy();
                    let dist_to_ball = ball_pos.distance(player_pos);
                    let can_pickup = dist_to_ball <= PICKUP_RADIUS;
                    if can_pickup {
                        if ball.is_held() {
                            // We can't steal the ball.
                        } else {
                            // Take the ball (regardless of if we claimed it).
                            ball.pick_up(&mut ball_tfm, &mut ball_groups);
                            player.holding_ball = true;
                            commands.entity(player_entity).push_children(&[ball_entity]);

                            // TODO: regroup with squad
                        }
                        player.claimed_ball = false;
                        player.chasing_ball = None;
                    } else {
                        // We haven't arrived at the ball yet. Just keep running.
                        let run_direction = (ball_pos - player_pos).normalize();
                        accum_linvel += CHASE_FACTOR * squad_stats.run_speed * run_direction;

                        if !player.claimed_ball && dist_to_ball < CLAIM_RADIUS {
                            if ball.claim(player.claimant_group_mask()) {
                                player.claimed_ball = true;
                            } else {
                                // Someone already claimed this ball, so search the area for more balls.
                                player.chasing_ball = None;
                            }
                        }
                    }
                }
            }

            let select_all_players = QueryFilter::new().groups(CollisionGroups::new(
                collision::groups::QUERY,
                collision::groups::PLAYER,
            ));
            let mut n_players_nearby = 0;
            let mut sum_nearby_dist = Vec2::ZERO;
            rapier_context.intersections_with_shape(
                player_pos,
                0.0,
                &Collider::ball(AVOID_RADIUS),
                select_all_players,
                |other_player_entity| {
                    n_players_nearby += 1;
                    let Ok(other_player_transform) = player_transforms.get(other_player_entity)
                    else {
                        return true;
                    };
                    sum_nearby_dist += player_pos - other_player_transform.translation().xy();
                    true
                },
            );
            if n_players_nearby > 0 {
                accum_linvel += AVOID_FACTOR * (sum_nearby_dist / n_players_nearby as f32);
            }

            assert!(accum_linvel.is_finite());
            if accum_linvel.length_squared() > 0.0 {
                player_velocity.linvel = accum_linvel;
                player_velocity.linvel = squad_stats.run_speed * player_velocity.linvel.normalize();
            }
            assert!(player_velocity.linvel.is_finite(), "{accum_linvel}");
        }
    }

    fn claimant_group_mask(&self) -> u8 {
        debug_assert!(self.team < 8);
        1 << self.team
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
