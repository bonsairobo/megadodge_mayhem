use super::PlayerBall;
use crate::{
    ball::{Ball, BallAssets},
    boundaries::Boundaries,
    parameters::DESPAWN_SECONDS,
};
use bevy::prelude::*;
use std::time::Duration;

#[derive(Component)]
pub struct KnockedOut;

impl KnockedOut {
    pub fn update(
        mut commands: Commands,
        time: Res<Time>,
        bounds: Res<Boundaries>,
        ball_assets: Res<BallAssets>,
        mut players: Query<
            (Entity, &GlobalTransform, &mut PlayerBall, &mut DespawnTimer),
            With<KnockedOut>,
        >,
    ) {
        for (entity, tfm, mut ball, mut timer) in &mut players {
            if ball.holding_ball {
                // Drop the ball.
                // TODO: preserve the player's original ball and make it dynamic?
                let position = tfm.translation();
                Ball::spawn_on_ground(&mut commands, &ball_assets, &bounds, position);
                commands.entity(entity).despawn_descendants();
                ball.holding_ball = false;
            }

            timer.timer.tick(time.delta());
            if timer.timer.finished() {
                commands.entity(entity).despawn();
            }
        }
    }
}

#[derive(Component)]
pub struct DespawnTimer {
    timer: Timer,
}

impl DespawnTimer {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::from_secs(DESPAWN_SECONDS), TimerMode::Once),
        }
    }
}
