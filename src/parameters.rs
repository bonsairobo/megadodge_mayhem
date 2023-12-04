/// How much to avoid other players.
pub const AVOID_FACTOR: f32 = 0.8;
/// Try to avoid other players in some radius.
pub const AVOID_RADIUS: f32 = 30.0;
/// How much to chase a ball.
pub const CHASE_FACTOR: f32 = 0.2;
/// If we're close enough, claim the ball (for this team).
pub const CLAIM_RADIUS: f32 = 50.0;
/// How close a player must be to a ball to pick it up.
pub const PICKUP_RADIUS: f32 = 10.0;
/// Make sure the ball doesn't immediately collide with the thrower.
pub const THROW_START_RADIUS: f32 = 25.0;
