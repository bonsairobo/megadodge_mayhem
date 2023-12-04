use bevy::prelude::Vec2;

#[derive(Clone, Copy)]
pub struct Aabb2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb2 {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }
}
