use crate::collision;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{Collider, CollisionGroups, RigidBody};

#[derive(Component)]
pub struct Gym;

impl Gym {
    pub fn spawn(commands: &mut Commands, assets: &GymAssets) {
        let params = &assets.params;
        let he = params.half_extents();
        let ht = params.half_thickness();

        // Make 5 cuboids to contain the gym. The floor is opaque and the walls
        // are transparent.

        // To keep the top of the floor at Y=0.0.
        let y_offset = -ht;

        // floor
        commands.spawn((
            PbrBundle {
                mesh: assets.floor_mesh.clone(),
                material: assets.floor_material.clone(),
                transform: Transform::from_translation(Vec3::new(0.0, y_offset, 0.0)),
                ..default()
            },
            RigidBody::Fixed,
            Collider::cuboid(he.x, ht, he.z),
            CollisionGroups::new(
                collision::groups::BOUNDARIES,
                collision::groups::PLAYER
                    | collision::groups::GROUND_BALL
                    | collision::groups::THROWN_BALL,
            ),
        ));

        let walls = [
            (
                Vec3::new(-he.x, he.y + y_offset, 0.0),
                Collider::cuboid(ht, he.y, he.z),
            ),
            (
                Vec3::new(he.x, he.y + y_offset, 0.0),
                Collider::cuboid(ht, he.y, he.z),
            ),
            (
                Vec3::new(0.0, he.y + y_offset, -he.z),
                Collider::cuboid(he.x, he.y, ht),
            ),
            (
                Vec3::new(0.0, he.y + y_offset, he.z),
                Collider::cuboid(he.x, he.y, ht),
            ),
        ];
        for (position, collider) in walls {
            commands.spawn((
                Transform::from_translation(position),
                GlobalTransform::default(),
                RigidBody::Fixed,
                collider,
                CollisionGroups::new(
                    collision::groups::BOUNDARIES,
                    collision::groups::PLAYER
                        | collision::groups::GROUND_BALL
                        | collision::groups::THROWN_BALL,
                ),
            ));
        }
    }
}

#[derive(Clone, Copy)]
pub struct GymParams {
    pub size: Vec3,
    pub thickness: f32,
}

impl Default for GymParams {
    fn default() -> Self {
        Self {
            size: Vec3::new(20.0, 100.0, 36.0),
            thickness: 0.1,
        }
    }
}

impl GymParams {
    pub fn half_extents(&self) -> Vec3 {
        0.5 * self.size
    }

    pub fn half_thickness(&self) -> f32 {
        0.5 * self.thickness
    }
}

pub struct GymAssets {
    pub params: GymParams,
    pub floor_mesh: Handle<Mesh>,
    pub floor_material: Handle<StandardMaterial>,
}

impl GymAssets {
    pub fn new(
        params: GymParams,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let he = params.half_extents();
        let ht = params.half_thickness();
        Self {
            params,
            floor_mesh: meshes.add(
                shape::Box {
                    min_x: -he.x,
                    max_x: he.x,
                    min_y: -ht,
                    max_y: ht,
                    min_z: -he.z,
                    max_z: he.z,
                }
                .try_into()
                .unwrap(),
            ),
            floor_material: materials.add(Color::GRAY.into()),
        }
    }
}
