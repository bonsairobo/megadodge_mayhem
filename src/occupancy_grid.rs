use crate::{
    aabb::Aabb2,
    grid2::Grid2,
    player::{KnockedOut, Player},
};
use bevy::prelude::*;

#[derive(Resource)]
pub struct OccupancyGrid {
    pub players_in_cell: Grid2<u8>,
    pub cell_size: Vec2,
    pub min: Vec2,
}

impl OccupancyGrid {
    pub fn new(cell_size: Vec2, aabb: Aabb2) -> Self {
        let pad = cell_size;
        let grid_shape = ((aabb.shape() + 2.0 * pad) / cell_size).as_uvec2();
        OccupancyGrid {
            players_in_cell: Grid2::new_fill(grid_shape, 0),
            cell_size,
            min: aabb.min - pad,
        }
    }

    pub fn update(
        mut grid: ResMut<Self>,
        players: Query<&GlobalTransform, (With<Player>, Without<KnockedOut>)>,
    ) {
        grid.players_in_cell.fill(0);
        grid.players_in_cell.fill_boundary(u8::MAX);
        for tfm in &players {
            let grid_p = grid.position(tfm);
            let cell = grid_p.as_ivec2();
            let n = &mut grid.players_in_cell[cell];
            *n = n.saturating_add(1);
        }
    }

    pub fn position(&self, tfm: &GlobalTransform) -> Vec2 {
        (tfm.translation().xz() - self.min) / self.cell_size
    }
}
