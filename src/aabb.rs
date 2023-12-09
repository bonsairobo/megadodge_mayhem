use bevy::prelude::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Aabb2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb2 {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn center(&self) -> Vec2 {
        0.5 * (self.min + self.max)
    }

    pub fn shape(&self) -> Vec2 {
        self.max - self.min
    }
}
