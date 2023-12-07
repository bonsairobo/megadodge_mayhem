use super::{KnockedOut, Player};
use crate::{collision, parameters::AVOID_RADIUS};
use bevy::prelude::*;
use bevy_rapier3d::prelude::{Collider, CollisionGroups, QueryFilter, RapierContext};

#[derive(Component, Default)]
pub struct AvoidPlayers {
    pub nearby_players_mass: Vec3,
}

impl AvoidPlayers {
    // PERF: this shows up in traces as being a bit expensive. Not sure how to
    // improve it though without making players RigidBody::Dynamic, which might
    // have bad side effects.
    pub fn avoid_other_players(
        rapier_context: Res<RapierContext>,
        mut avoiders: Query<(&mut Self, &GlobalTransform), Without<KnockedOut>>,
        player_transforms: Query<&GlobalTransform, (With<Player>, Without<KnockedOut>)>,
    ) {
        avoiders.par_iter_mut().for_each(|(mut avoider, tfm)| {
            avoider.nearby_players_mass = Vec3::ZERO;
            let position = tfm.translation();
            let select_all_players = QueryFilter::new().groups(CollisionGroups::new(
                collision::groups::QUERY,
                collision::groups::PLAYER,
            ));
            let mut n_players_nearby = 0;
            let mut sum_nearby_dist = Vec3::ZERO;
            rapier_context.intersections_with_shape(
                position,
                default(),
                &Collider::ball(AVOID_RADIUS),
                select_all_players,
                |other_player_entity| {
                    n_players_nearby += 1;
                    let Ok(other_player_transform) = player_transforms.get(other_player_entity)
                    else {
                        return true;
                    };
                    sum_nearby_dist += position - other_player_transform.translation();
                    true
                },
            );
            if n_players_nearby > 0 {
                avoider.nearby_players_mass = sum_nearby_dist / n_players_nearby as f32;
            }
        });
    }
}
