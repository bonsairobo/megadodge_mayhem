use bevy::prelude::Vec2;

/// How much to avoid other players.
pub const AVOID_FACTOR: f32 = 0.8;
/// Try to avoid other players in some radius.
pub const AVOID_RADIUS: f32 = 0.3;
pub const BLOOM_INTENSITY: f32 = 1.5;
pub const CHASE_FACTOR: f32 = 0.2;
pub const DESPAWN_SECONDS: u64 = 5;
pub const OCCUPANCY_CELL_SIZE: Vec2 = Vec2::splat(AVOID_RADIUS);
pub const SQUAD_AI_COLLIDER_HEIGHT: f32 = 0.1;
pub const SQUAD_AI_COLLIDER_RADIUS: f32 = 2.0;
/// Players per square meter.
pub const SQUAD_CLUSTER_DENSITY: f32 = 4.0;
/// How far a thrown ball travels upwards on its trajectory.
///
/// This should be nonzero to avoid friendly fire.
pub const THROW_LOFT: f32 = 0.1;
/// A percentage of the target's height.
pub const THROW_TARGET_HEIGHT: f32 = 0.8;
/// How long a player must wait between throws.
///
/// This doubles as the time a player must wait between picking up a ball and
/// throwing it.
pub const THROW_COOLDOWN_MILLIS: u64 = 2000;
pub const THROW_OVER_HEAD: f32 = 0.3;
