use crate::geometry::Aabb2;
use bevy::prelude::{Resource, Vec3, Vec3Swizzles};

#[derive(Resource)]
pub struct Boundaries {
    pub min: Vec3,
    pub max: Vec3,
}

impl Boundaries {
    pub fn aabb2(&self) -> Aabb2 {
        Aabb2::new(self.min.xz(), self.max.xz())
    }
}
