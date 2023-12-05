use super::KnockedOut;
use crate::{collision, team::Team};
use bevy::prelude::*;
use bevy_rapier3d::prelude::{CollisionGroups, QueryFilter, RapierContext};

#[derive(Component, Default)]
pub struct TargetEnemy {
    pub target_enemy: Option<Entity>,
    pub chase_vector: Vec3,
    pub can_target: bool,
}

impl TargetEnemy {
    pub fn find_target_enemy(
        rapier_context: Res<RapierContext>,
        mut players: Query<(&mut Self, &Team, &GlobalTransform), Without<KnockedOut>>,
        teams: Query<&Team>,
    ) {
        for (mut player_target, player_team, player_tfm) in &mut players {
            player_target.target_enemy = None;

            if !player_target.can_target {
                continue;
            }

            let player_position = player_tfm.translation();

            let entity_on_enemy_team = |entity| {
                let Ok(team) = teams.get(entity) else {
                    return false;
                };
                team.team() != player_team.team()
            };
            let select_enemy_players = QueryFilter::new()
                .groups(CollisionGroups::new(
                    collision::groups::QUERY,
                    collision::groups::PLAYER,
                ))
                .predicate(&entity_on_enemy_team);

            // TODO: maybe use claimant bits for this too? It's not
            // necessarily bad for two players to aim at the same target.

            // Identify the closest target.
            if let Some((nearest_player_entity, _projection)) =
                rapier_context.project_point(player_position, true, select_enemy_players)
            {
                player_target.target_enemy = Some(nearest_player_entity);
            }
        }
    }
}
