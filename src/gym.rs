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

        // Make 6 cuboids to contain the gym. The floor is opaque and the walls
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

        // min/max X walls
        let positions = [
            Vec3::new(-he.x, y_offset, 0.0),
            Vec3::new(he.x, y_offset, 0.0),
        ];
        for position in positions {
            commands.spawn((
                PbrBundle {
                    mesh: assets.x_wall_mesh.clone(),
                    material: assets.wall_material.clone(),
                    transform: Transform::from_translation(position),
                    ..default()
                },
                RigidBody::Fixed,
                Collider::cuboid(ht, he.y, he.z),
                CollisionGroups::new(
                    collision::groups::BOUNDARIES,
                    collision::groups::PLAYER
                        | collision::groups::GROUND_BALL
                        | collision::groups::THROWN_BALL,
                ),
            ));
        }
        // min/max Z walls
        let positions = [
            Vec3::new(0.0, y_offset, -he.z),
            Vec3::new(0.0, y_offset, he.z),
        ];
        for position in positions {
            commands.spawn((
                PbrBundle {
                    mesh: assets.z_wall_mesh.clone(),
                    material: assets.wall_material.clone(),
                    transform: Transform::from_translation(position),
                    ..default()
                },
                RigidBody::Fixed,
                Collider::cuboid(he.x, he.y, ht),
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
    pub wall_alpha: f32,
}

impl Default for GymParams {
    fn default() -> Self {
        Self {
            size: Vec3::new(500.0, 500.0, 800.0),
            thickness: 10.0,
            wall_alpha: 0.1,
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
    pub x_wall_mesh: Handle<Mesh>,
    pub z_wall_mesh: Handle<Mesh>,
    pub floor_material: Handle<StandardMaterial>,
    pub wall_material: Handle<StandardMaterial>,
}

impl GymAssets {
    pub fn new(
        params: GymParams,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let he = params.half_extents();
        let ht = params.half_thickness();
        // TODO: it's a little silly to use a distinct Box mesh for different
        // wall sizes, but it's the simplest solution for now
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
            x_wall_mesh: meshes.add(
                shape::Box {
                    min_x: -ht,
                    max_x: ht,
                    min_y: -he.y,
                    max_y: he.y,
                    min_z: -he.z,
                    max_z: he.z,
                }
                .try_into()
                .unwrap(),
            ),
            z_wall_mesh: meshes.add(
                shape::Box {
                    min_x: -he.x,
                    max_x: he.x,
                    min_y: -he.y,
                    max_y: he.y,
                    min_z: -ht,
                    max_z: ht,
                }
                .try_into()
                .unwrap(),
            ),
            floor_material: materials.add(Color::GRAY.into()),
            wall_material: materials.add(Color::WHITE.with_a(params.wall_alpha).into()),
        }
    }
}
