use crate::{aabb::Aabb2, collision};
use bevy::prelude::*;
use bevy_mod_picking::prelude::RapierPickable;
use bevy_rapier3d::prelude::{Collider, CollisionGroups, RigidBody};

#[derive(Component)]
pub struct Gym;

#[derive(Component)]
pub struct Floor;

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
            Floor,
            Gym,
            PbrBundle {
                mesh: assets.floor_mesh.clone(),
                material: assets.floor_material.clone(),
                transform: Transform::from_translation(Vec3::new(0.0, y_offset, 0.0)),
                ..default()
            },
            RapierPickable,
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
                Vec3::new(-he.x - ht, he.y + y_offset, 0.0),
                Collider::cuboid(ht, he.y, he.z),
            ),
            (
                Vec3::new(he.x + ht, he.y + y_offset, 0.0),
                Collider::cuboid(ht, he.y, he.z),
            ),
            (
                Vec3::new(0.0, he.y + y_offset, -he.z - ht),
                Collider::cuboid(he.x, he.y, ht),
            ),
            (
                Vec3::new(0.0, he.y + y_offset, he.z + ht),
                Collider::cuboid(he.x, he.y, ht),
            ),
        ];
        for (position, collider) in walls {
            commands.spawn((
                Gym,
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
            size: Vec3::new(40.0, 100.0, 72.0),
            thickness: 1.0,
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

    pub fn spawn_aabbs(&self) -> [Aabb2; 2] {
        let he = self.half_extents();
        [
            Aabb2::new([-he.x, he.z - 1.0].into(), [he.x, he.z + 1.0].into()),
            Aabb2::new([-he.x, -he.z - 1.0].into(), [he.x, -he.z + 1.0].into()),
        ]
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
