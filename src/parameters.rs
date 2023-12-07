/// How much to avoid other players.
pub const AVOID_FACTOR: f32 = 0.8;
/// Try to avoid other players in some radius.
pub const AVOID_RADIUS: f32 = 0.3;
pub const BLOOM_INTENSITY: f32 = 1.5;
pub const CHASE_FACTOR: f32 = 0.2;
/// If we're close enough, claim the ball (for this team).
// pub const CLAIM_RADIUS: f32 = 2.0;
pub const DESPAWN_SECONDS: u64 = 5;
/// How close a player must be to a ball to pick it up.
pub const PICKUP_RADIUS: f32 = 1.0;
pub const SQUAD_AI_COLLIDER_HEIGHT: f32 = 0.1;
/// Players per square meter.
pub const SQUAD_CLUSTER_DENSITY: f32 = 1.0;
/// How long a player must wait between throws.
///
/// This doubles as the time a player must wait between picking up a ball and
/// throwing it.
pub const THROW_COOLDOWN_MILLIS: u64 = 2000;
