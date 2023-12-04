use crate::collision;
use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use bevy_rapier2d::prelude::*;
use std::time::Duration;

#[derive(Component, Default)]
pub struct Ball {
    /// When a group's bit is set, then some player has already claimed this
    /// ball.
    claims_mask: u8,
    is_held: bool,
}

// TODO: differentiate (visually) between balls on ground and in air

impl Ball {
    pub const DEPTH_LAYER: f32 = 100.0;

    pub fn is_held(&self) -> bool {
        self.is_held
    }

    fn ground_groups() -> CollisionGroups {
        CollisionGroups::new(collision::groups::GROUND_BALL, collision::groups::QUERY)
    }

    fn thrown_groups() -> CollisionGroups {
        CollisionGroups::new(
            collision::groups::PLAYER | collision::groups::THROWN_BALL,
            collision::groups::PLAYER,
        )
    }

    pub fn spawn_on_ground(commands: &mut Commands, ball_assets: &BallAssets, position: Vec2) {
        commands.spawn((
            Self::default(),
            MaterialMesh2dBundle {
                mesh: ball_assets.mesh.clone(),
                material: ball_assets.material.clone(),
                transform: Transform::from_translation(position.extend(Self::DEPTH_LAYER)),
                ..default()
            },
            RigidBody::KinematicPositionBased,
            Collider::ball(ball_assets.radius),
            Self::ground_groups(),
        ));
    }

    pub fn spawn_multiple_in_line(
        commands: &mut Commands,
        ball_assets: &BallAssets,
        n_balls: usize,
        start: Vec2,
        end: Vec2,
    ) {
        let delta = (end - start) / n_balls as f32;
        let mut position = start;
        for _ in 0..n_balls {
            Self::spawn_on_ground(commands, ball_assets, position);
            position += delta;
        }
    }

    pub fn pick_up(&mut self, transform: &mut Transform, groups: &mut CollisionGroups) {
        self.clear_claims();
        self.is_held = true;
        // TODO: this is going to cause flickering because the transform will be
        // applied before the player can become the parent
        transform.translation = Vec3 {
            z: Self::DEPTH_LAYER,
            ..default()
        };
        // Don't appear in any collisions or queries until the ball is thrown.
        groups.filters = Group::NONE;
    }

    /// Returns `true` if the claim was successful.
    pub fn claim(&mut self, mask: u8) -> bool {
        let already_claimed = self.is_claimed(mask);
        self.claims_mask |= mask;
        !already_claimed
    }

    pub fn drop_claim(&mut self, mask: u8) {
        self.claims_mask &= !mask;
    }

    pub fn is_claimed(&self, mask: u8) -> bool {
        (self.claims_mask & mask) != 0
    }

    pub fn clear_claims(&mut self) {
        self.claims_mask = 0;
    }

    pub fn spawn_thrown(
        commands: &mut Commands,
        ball_assets: &BallAssets,
        position: Vec2,
        velocity: Vec2,
    ) {
        commands.spawn((
            Self::default(),
            MaterialMesh2dBundle {
                mesh: ball_assets.mesh.clone(),
                material: ball_assets.material.clone(),
                transform: Transform::from_translation(position.extend(Self::DEPTH_LAYER)),
                ..default()
            },
            RigidBody::Dynamic,
            Collider::ball(ball_assets.radius),
            Self::thrown_groups(),
            Ccd::enabled(),
            ActiveEvents::COLLISION_EVENTS,
            ColliderMassProperties::Density(0.5),
            Velocity::linear(velocity),
            Damping {
                // Air friction.
                linear_damping: 0.8,
                angular_damping: 1.0,
            },
            Restitution {
                // Bouncy
                coefficient: 0.9,
                ..default()
            },
            Friction {
                coefficient: 0.7,
                ..default()
            },
            // After the timer is done, the ball can be picked up again.
            Cooldown {
                timer: Timer::new(Duration::from_secs(2), TimerMode::Once),
            },
        ));
    }

    pub fn handle_cooldown(
        mut commands: Commands,
        time: Res<Time>,
        mut cooling_balls: Query<
            (Entity, &mut Cooldown, &mut RigidBody, &mut CollisionGroups),
            With<Ball>,
        >,
    ) {
        for (entity, mut cool, mut body, mut groups) in cooling_balls.iter_mut() {
            cool.timer.tick(time.delta());
            if cool.timer.finished() {
                *body = RigidBody::KinematicPositionBased;
                *groups = Self::ground_groups();
                commands
                    .entity(entity)
                    .remove::<(ActiveEvents, Cooldown, Ccd)>();
            }
        }
    }
}

#[derive(Resource)]
pub struct BallAssets {
    pub radius: f32,
    pub mesh: Mesh2dHandle,
    pub material: Handle<ColorMaterial>,
}

impl BallAssets {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<ColorMaterial>) -> Self {
        let radius = 4.0;
        Self {
            radius,
            mesh: meshes.add(shape::Circle::new(radius).into()).into(),
            material: materials.add(ColorMaterial::from(Color::RED)),
        }
    }
}

#[derive(Component)]
pub struct Cooldown {
    pub timer: Timer,
}
