use crate::{ball::Ball, player::Player};
use bevy::prelude::{EventReader, Query};
use bevy_rapier2d::prelude::CollisionEvent;

pub mod groups {
    use bevy_rapier2d::prelude::Group;

    pub const QUERY: Group = Group::GROUP_1;
    pub const PLAYER: Group = Group::GROUP_2;
    pub const GROUND_BALL: Group = Group::GROUP_3;
    pub const THROWN_BALL: Group = Group::GROUP_4;
}

pub fn handle_collision_events(
    mut events: EventReader<CollisionEvent>,
    players: Query<&Player>,
    balls: Query<&Ball>,
) {
    for event in events.read() {
        if let &CollisionEvent::Started(mut player_entity, mut ball_entity, _flags) = event {
            if !players.contains(player_entity) {
                std::mem::swap(&mut player_entity, &mut ball_entity);
            }
            if !players.contains(player_entity) || !balls.contains(ball_entity) {
                // Only handle player on ball events.
                continue;
            }

            // Player got hit by thrown ball. Let's see if they can catch it.
            // println!("player hit");
        }
    }
}
