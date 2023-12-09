use crate::{boundaries::Boundaries, collision, geometry::Aabb2};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

#[derive(Component, Default)]
pub struct Ball {
    is_held: bool,
    /// Will knock a player out if hit.
    dangerous: bool,
}

#[derive(Bundle)]
pub struct BallBundle {
    pub active_events: ActiveEvents,
    pub ball: Ball,
    pub body: RigidBody,
    pub collider: Collider,
    pub collision_types: ActiveCollisionTypes,
    pub groups: CollisionGroups,
    pub pbr: PbrBundle,
}

impl BallBundle {
    pub fn new_on_ground(
        ball_assets: &BallAssets,
        bounds: &Boundaries,
        mut position: Vec3,
    ) -> Self {
        position.y = ball_assets.radius;
        Self {
            active_events: ActiveEvents::COLLISION_EVENTS,
            ball: default(),
            body: RigidBody::KinematicPositionBased,
            collider: Collider::ball(ball_assets.radius),
            collision_types: ActiveCollisionTypes::default()
                | ActiveCollisionTypes::KINEMATIC_KINEMATIC,
            groups: Ball::ground_groups(),
            pbr: PbrBundle {
                mesh: ball_assets.mesh.clone(),
                material: ball_assets.material.clone(),
                transform: Transform::from_translation(position.clamp(bounds.min, bounds.max)),
                ..default()
            },
        }
    }

    fn new_thrown(ball_assets: &BallAssets, position: Vec3) -> Self {
        Self {
            ball: Ball {
                dangerous: true,
                ..default()
            },
            body: RigidBody::Dynamic,
            collider: Collider::ball(ball_assets.radius),
            groups: Ball::thrown_groups(),
            active_events: ActiveEvents::COLLISION_EVENTS,
            collision_types: ActiveCollisionTypes::default()
                | ActiveCollisionTypes::KINEMATIC_KINEMATIC,
            pbr: PbrBundle {
                mesh: ball_assets.mesh.clone(),
                material: ball_assets.material.clone(),
                transform: Transform::from_translation(position),
                ..default()
            },
        }
    }
}

#[derive(Bundle)]
pub struct ThrownBallBundle {
    pub ball: BallBundle,
    pub ccd: Ccd,
    pub damping: Damping,
    pub friction: Friction,
    pub mass: ColliderMassProperties,
    pub restitution: Restitution,
    pub velocity: Velocity,
}

impl ThrownBallBundle {
    pub fn new(ball_assets: &BallAssets, position: Vec3, velocity: Vec3) -> Self {
        Self {
            ball: BallBundle::new_thrown(ball_assets, position),
            ccd: Ccd::enabled(),
            damping: Damping {
                // Balls should come to rest eventually.
                linear_damping: 0.1,
                angular_damping: 0.1,
            },
            friction: Friction {
                coefficient: 0.7,
                ..default()
            },
            // Increased density for better impact.
            mass: ColliderMassProperties::Density(10.0),
            restitution: Restitution {
                // Bouncy
                coefficient: 0.9,
                ..default()
            },
            velocity: Velocity::linear(velocity),
        }
    }
}

impl Ball {
    const HELD_OFFSET: Vec3 = Vec3::new(0.4, 0.0, 0.0);

    pub fn is_held(&self) -> bool {
        self.is_held
    }

    pub fn is_dangerous(&self) -> bool {
        self.dangerous
    }

    pub fn ground_groups() -> CollisionGroups {
        CollisionGroups::new(
            collision::groups::GROUND_BALL,
            collision::groups::QUERY
                | collision::groups::PLAYER
                | collision::groups::BOUNDARIES
                | collision::groups::GROUND_BALL,
        )
    }

    fn thrown_groups() -> CollisionGroups {
        CollisionGroups::new(
            collision::groups::THROWN_BALL,
            // WARNING/HACK: Do not collide with ground balls.
            //
            // While this would be nice, it's important that balls hit the
            // ground to trigger a state transition. There are some cases where
            // ground balls will congregate inside of a squad cluster, and
            // they form a little "crown" where a thrown ball can rest without
            // hitting the ground.
            //
            // The way that knocked out players drop balls is partly to blame
            // for this issue.
            collision::groups::PLAYER
                | collision::groups::BOUNDARIES
                | collision::groups::THROWN_BALL,
        )
    }

    pub fn spawn_multiple_in_aabb(
        commands: &mut Commands,
        ball_assets: &BallAssets,
        bounds: &Boundaries,
        aabb: Aabb2,
        n_balls: u32,
    ) {
        let mut rng = rand::thread_rng();
        for _ in 0..n_balls {
            let x = rng.gen_range(aabb.min.x..aabb.max.x);
            let z = rng.gen_range(aabb.min.y..aabb.max.y);
            commands.spawn(BallBundle::new_on_ground(
                ball_assets,
                bounds,
                Vec3::new(x, 0.0, z),
            ));
        }
    }

    pub fn pick_up(
        &mut self,
        transform: &mut Transform,
        body: &mut RigidBody,
        groups: &mut CollisionGroups,
    ) {
        self.is_held = true;
        // TODO: this is going to cause flickering because the transform will be
        // applied before the player can become the parent
        transform.translation = Self::HELD_OFFSET;
        *body = RigidBody::KinematicPositionBased;
        // Don't appear in any collisions or queries until the ball is thrown.
        groups.filters = Group::NONE;
    }

    pub fn on_touch_ground(&mut self) {
        self.dangerous = false;
    }
}

#[derive(Resource)]
pub struct BallAssets {
    pub radius: f32,
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

impl BallAssets {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        let radius = 0.2;
        Self {
            radius,
            mesh: meshes.add(
                shape::Icosphere {
                    radius,
                    subdivisions: 5,
                }
                .try_into()
                .unwrap(),
            ),
            material: materials.add(Color::RED.into()),
        }
    }
}
