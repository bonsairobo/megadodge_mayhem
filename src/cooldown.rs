use bevy::{prelude::Component, time::Timer};

#[derive(Component)]
pub struct Cooldown {
    pub timer: Timer,
}
