use bevy::prelude::{IVec2, UVec2};
use std::ops::{Index, IndexMut};

pub struct Grid2<T> {
    cells: Vec<T>,
    shape: UVec2,
    max: IVec2,
}

impl<T: Clone> Grid2<T> {
    pub fn new_fill(shape: UVec2, value: T) -> Self {
        Self {
            cells: vec![value; (shape.x * shape.y) as usize],
            shape,
            max: (shape - UVec2::ONE).as_ivec2(),
        }
    }

    pub fn fill(&mut self, value: T) {
        self.cells.fill(value);
    }
}

impl<T> Index<IVec2> for Grid2<T> {
    type Output = T;

    fn index(&self, index: IVec2) -> &Self::Output {
        let i = index.clamp(IVec2::ZERO, self.max);
        &self.cells[(i.y * self.shape.x as i32 + i.x) as usize]
    }
}

impl<T> IndexMut<IVec2> for Grid2<T> {
    fn index_mut(&mut self, index: IVec2) -> &mut Self::Output {
        let i = index.clamp(IVec2::ZERO, self.max);
        &mut self.cells[(i.y * self.shape.x as i32 + i.x) as usize]
    }
}
