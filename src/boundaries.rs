use bevy::prelude::{Resource, Vec3};

#[derive(Resource)]
pub struct Boundaries {
    pub min: Vec3,
    pub max: Vec3,
}
