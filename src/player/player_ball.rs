use super::{KnockedOut, Player, ThrowCooldown};
use crate::{
    ball::Ball,
    collision,
    parameters::{CLAIM_RADIUS, PICKUP_RADIUS},
    team::Team,
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

#[derive(Component, Default)]
pub struct PlayerBall {
    pub target_ball: Option<Entity>,
    pub chase_vector: Vec3,
    pub claimed_ball: bool,
    pub holding_ball: bool,
}

impl PlayerBall {
    #[allow(clippy::complexity)]
    pub fn choose_target_ball(
        rapier_context: Res<RapierContext>,
        mut players: Query<(&Team, &GlobalTransform, &mut Self), Without<KnockedOut>>,
        mut balls: Query<&mut Ball>,
    ) {
        for (team, _tfm, mut target) in &mut players {
            // Always drop our claim before checking for a new ball to chase.
            let old_target = target.target_ball.take();
            if target.claimed_ball {
                if let Some(old_chasing) = old_target {
                    if let Ok(mut ball) = balls.get_mut(old_chasing) {
                        ball.drop_claim(team.claimant_group_mask());
                        target.claimed_ball = false;
                    }
                }
            }
        }

        players.par_iter_mut().for_each(|(team, tfm, mut chase)| {
            if chase.holding_ball {
                return;
            }

            let position = tfm.translation();

            // Find the nearest unclaimed ball and chase it.
            let can_chase_ball = |ball_entity| {
                let Ok(ball) = balls.get(ball_entity) else {
                    return false;
                };
                !ball.is_claimed(team.claimant_group_mask())
            };
            let select_ground_balls = QueryFilter::new()
                .groups(CollisionGroups::new(
                    collision::groups::QUERY,
                    collision::groups::GROUND_BALL,
                ))
                .predicate(&can_chase_ball);
            let maybe_nearest_entity = rapier_context
                .project_point(position, true, select_ground_balls)
                .map(|(e, _)| e);

            chase.target_ball = maybe_nearest_entity;
        });
    }

    // #[allow(clippy::complexity)]
    // pub fn chase_ball(
    //     mut commands: Commands,
    //     mut players: Query<
    //         (
    //             Entity,
    //             &Team,
    //             &GlobalTransform,
    //             &mut Self,
    //             &mut ThrowCooldown,
    //         ),
    //         (With<Player>, Without<KnockedOut>),
    //     >,
    //     mut balls: Query<
    //         (
    //             Entity,
    //             &GlobalTransform,
    //             &mut Ball,
    //             &mut Transform,
    //             &mut RigidBody,
    //             &mut CollisionGroups,
    //         ),
    //         Without<Player>,
    //     >,
    // ) {
    //     for (chaser_entity, team, global_tfm, mut player_ball, mut throw_cooldown) in &mut players {
    //         let Some(chasing_ball_entity) = player_ball.target_ball else {
    //             continue;
    //         };

    //         let Ok((
    //             ball_entity,
    //             ball_global_tfm,
    //             mut ball,
    //             mut ball_tfm,
    //             mut ball_body,
    //             mut ball_groups,
    //         )) = balls.get_mut(chasing_ball_entity)
    //         else {
    //             continue;
    //         };

    //         let player_pos = global_tfm.translation();

    //         // Check if the player can pick up the ball.
    //         let ball_pos = ball_global_tfm.translation();
    //         let dist_to_ball = ball_pos.distance(player_pos);
    //         let can_pickup = dist_to_ball <= PICKUP_RADIUS;
    //         if can_pickup {
    //             if ball.is_held() {
    //                 // We can't steal the ball.
    //             } else {
    //                 // Take the ball (regardless of if we claimed it).
    //                 ball.pick_up(&mut ball_tfm, &mut ball_body, &mut ball_groups);
    //                 player_ball.holding_ball = true;
    //                 throw_cooldown.timer.reset();
    //                 commands.entity(chaser_entity).push_children(&[ball_entity]);

    //                 // TODO: regroup with squad
    //             }
    //             player_ball.claimed_ball = false;
    //             player_ball.target_ball = None;
    //         } else {
    //             // We haven't arrived at the ball yet. Just keep running.
    //             player_ball.chase_vector = (ball_pos - player_pos).normalize();

    //             if !player_ball.claimed_ball && dist_to_ball < CLAIM_RADIUS {
    //                 if ball.claim(team.claimant_group_mask()) {
    //                     player_ball.claimed_ball = true;
    //                 } else {
    //                     // Someone already claimed this ball, so search the area for more balls.
    //                     player_ball.target_ball = None;
    //                 }
    //             }
    //         }
    //     }
    // }

    #[allow(clippy::complexity)]
    pub fn pick_up_ball(
        mut commands: Commands,
        mut players: Query<
            (Entity, &GlobalTransform, &mut Self, &mut ThrowCooldown),
            (With<Player>, Without<KnockedOut>),
        >,
        mut balls: Query<
            (
                Entity,
                &GlobalTransform,
                &mut Ball,
                &mut Transform,
                &mut RigidBody,
                &mut CollisionGroups,
            ),
            Without<Player>,
        >,
    ) {
        for (chaser_entity, global_tfm, mut player_ball, mut throw_cooldown) in &mut players {
            let Some(target_ball_entity) = player_ball.target_ball else {
                continue;
            };

            let Ok((
                ball_entity,
                ball_global_tfm,
                mut ball,
                mut ball_tfm,
                mut ball_body,
                mut ball_groups,
            )) = balls.get_mut(target_ball_entity)
            else {
                continue;
            };

            let player_pos = global_tfm.translation();

            // Check if the player can pick up the ball.
            let ball_pos = ball_global_tfm.translation();
            let dist_to_ball = ball_pos.distance(player_pos);
            let can_pickup = dist_to_ball <= PICKUP_RADIUS;
            if can_pickup {
                if ball.is_held() {
                    // We can't steal the ball.
                } else {
                    // Take the ball (regardless of if we claimed it).
                    ball.pick_up(&mut ball_tfm, &mut ball_body, &mut ball_groups);
                    player_ball.holding_ball = true;
                    throw_cooldown.timer.reset();
                    commands.entity(chaser_entity).push_children(&[ball_entity]);
                }
                player_ball.claimed_ball = false;
                player_ball.target_ball = None;
            }
        }
    }
}
