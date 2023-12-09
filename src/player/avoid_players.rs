use super::{KnockedOut, Player};
use crate::occupancy_grid::OccupancyGrid;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct AvoidPlayers {
    pub nearby_players_mass: Vec3,
}

impl AvoidPlayers {
    #[allow(clippy::complexity)]
    pub fn avoid_other_players(
        grid: Res<OccupancyGrid>,
        mut players: Query<(&mut Self, &GlobalTransform), (With<Player>, Without<KnockedOut>)>,
    ) {
        players.par_iter_mut().for_each(|(mut avoider, tfm)| {
            let grid_p = grid.position(tfm);
            let cell = grid_p.as_ivec2();

            // TODO: use this?
            // let in_cell = grid.players_in_cell[cell];

            let mut n_players_nearby = 0u16;
            let mut center_of_mass = Vec2::ZERO;
            for [dx, dz] in [
                [-1, 0],
                [1, 0],
                [0, -1],
                [0, 1],
                // [-1, -1],
                // [-1, 1],
                // [1, -1],
                // [1, 1],
            ] {
                let d = IVec2::from([dx, dz]);
                let neighbor = cell + d;
                let n_players = grid.players_in_cell[neighbor];
                n_players_nearby += u16::from(n_players);

                let neighbor_center = neighbor.as_vec2() + Vec2::splat(0.5);
                center_of_mass += f32::from(n_players) * (neighbor_center - grid_p);

                // Simpler option that also seems to work.
                // center_of_mass += f32::from(n_players) * d.as_vec2();
            }

            avoider.nearby_players_mass = Vec3::ZERO;
            if n_players_nearby > 0 {
                avoider.nearby_players_mass =
                    -Vec3::new(center_of_mass.x, 0.0, center_of_mass.y) / n_players_nearby as f32;
            }
        });
    }
}
