use bevy::prelude::*;

pub type Probability = f32;

#[derive(Resource)]
pub struct AllStats {
    pub squads: Vec<PlayerStats>,
}

pub struct PlayerStats {
    pub dodge_speed: f32,
    pub run_speed: f32,
    pub walk_speed: f32,
    pub throw_accuracy: f32,
    pub throw_distance: f32,
    pub throw_speed: f32,

    // Idea: Heavy throwers need to stand still for a big throw, and it takes this long.
    // pub windup_seconds: f32,
    pub p_block: Probability,
    pub p_catch: Probability,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            dodge_speed: 150.0,
            run_speed: 100.0,
            walk_speed: 60.0,
            throw_accuracy: 0.8,
            throw_distance: 50.0,
            throw_speed: 300.0,
            p_block: 0.7,
            p_catch: 0.2,
        }
    }
}
