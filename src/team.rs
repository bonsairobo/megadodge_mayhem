use crate::{aabb::Aabb2, player::PlayerAssets, squad::Squad};
use bevy::prelude::*;

#[derive(Component)]
pub struct Team {
    team: u8,
}

impl Team {
    pub fn new(team: u8) -> Self {
        Self { team }
    }

    pub fn team(&self) -> u8 {
        self.team
    }

    pub fn spawn(
        commands: &mut Commands,
        assets: &PlayerAssets,
        team: u8,
        aabb: Aabb2,
        n_players: usize,
    ) {
        let squad = 0;
        Squad::spawn(commands, assets, team, squad, aabb, n_players);
    }
}

#[derive(Resource)]
pub struct TeamAssets {
    pub teams: Vec<PlayerAssets>,
}

impl Default for TeamAssets {
    fn default() -> Self {
        Self {
            teams: vec![
                PlayerAssets::new(Color::BLUE),
                PlayerAssets::new(Color::GREEN),
            ],
        }
    }
}
