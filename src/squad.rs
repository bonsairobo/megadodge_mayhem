use crate::{
    aabb::Aabb2,
    player::{Player, PlayerAssets},
};
use bevy::prelude::*;
use rand::Rng;

#[derive(Component)]
pub struct Squad;

impl Squad {
    pub fn spawn(
        commands: &mut Commands,
        assets: &PlayerAssets,
        team: u8,
        squad: u8,
        aabb: Aabb2,
        n_players: usize,
    ) {
        let mut rng = rand::thread_rng();
        for _ in 0..n_players {
            let x = rng.gen_range(aabb.min.x..aabb.max.x);
            let y = rng.gen_range(aabb.min.y..aabb.max.y);
            Player::spawn(commands, assets, team, squad, Vec2::new(x, y));
        }
    }
}
