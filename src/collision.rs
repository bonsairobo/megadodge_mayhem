use crate::{
    ball::{Ball, Cooldown},
    gym::Floor,
    player::{KnockedOut, Player},
    team::{AllTeamAssets, Team},
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::{ActiveEvents, Ccd, CollisionEvent, CollisionGroups, RigidBody};

pub mod groups {
    use bevy_rapier3d::prelude::Group;

    pub const QUERY: Group = Group::GROUP_1;
    pub const PLAYER: Group = Group::GROUP_2;
    pub const GROUND_BALL: Group = Group::GROUP_3;
    pub const THROWN_BALL: Group = Group::GROUP_4;
    pub const BOUNDARIES: Group = Group::GROUP_5;
}

#[allow(clippy::complexity)]
pub fn handle_ball_player_collisions(
    mut commands: Commands,
    team_assets: Res<AllTeamAssets>,
    mut events: EventReader<CollisionEvent>,
    mut players: Query<
        (
            &Team,
            &mut Player,
            &mut RigidBody,
            &mut CollisionGroups,
            &mut Handle<StandardMaterial>,
        ),
        Without<KnockedOut>,
    >,
    balls: Query<&Ball>,
) {
    for event in events.read() {
        let &CollisionEvent::Started(mut player_entity, mut ball_entity, _flags) = event else {
            continue;
        };

        if !players.contains(player_entity) {
            std::mem::swap(&mut player_entity, &mut ball_entity);
        }
        if !players.contains(player_entity) || !balls.contains(ball_entity) {
            // Only handle player on ball events.
            continue;
        }

        let Ok((player_team, mut player, mut player_body, mut player_groups, mut player_material)) =
            players.get_mut(player_entity)
        else {
            continue;
        };

        // Player got hit by thrown ball. Let's see if they can catch it.
        // println!("player hit");

        // Player failed to catch it, they are out.
        player.set_out(
            &mut commands,
            &team_assets,
            player_entity,
            player_team,
            &mut player_body,
            &mut player_groups,
            &mut player_material,
        );
    }
}

pub fn handle_ball_floor_collisions(
    mut commands: Commands,
    mut events: EventReader<CollisionEvent>,
    mut balls: Query<&mut CollisionGroups, With<Ball>>,
    floor: Query<Entity, With<Floor>>,
) {
    let Ok(floor_entity) = floor.get_single() else {
        return;
    };

    for event in events.read() {
        let &CollisionEvent::Started(mut this_floor_entity, mut ball_entity, _flags) = event else {
            continue;
        };

        if this_floor_entity != floor_entity {
            if ball_entity != floor_entity {
                continue;
            }
            std::mem::swap(&mut ball_entity, &mut this_floor_entity);
        }
        if !balls.contains(ball_entity) {
            // Only handle floor on ball events.
            continue;
        }

        let Ok(mut ball_groups) = balls.get_mut(ball_entity) else {
            continue;
        };

        *ball_groups = Ball::ground_groups();
        commands
            .entity(ball_entity)
            .remove::<(ActiveEvents, Cooldown, Ccd)>();
    }
}
