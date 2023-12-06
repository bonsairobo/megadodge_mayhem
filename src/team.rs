use bevy::prelude::*;

#[derive(Copy, Clone, Component)]
pub struct Team {
    team: u8,
}

impl Team {
    pub fn new(team: u8) -> Self {
        Self { team }
    }

    pub fn team(&self) -> u8 {
        self.team
    }

    pub fn claimant_group_mask(&self) -> u8 {
        debug_assert!(self.team < 8);
        1 << self.team
    }
}

#[derive(Resource)]
pub struct AllTeamAssets {
    pub teams: Vec<TeamAssets>,
}

impl AllTeamAssets {
    pub fn new(
        team_colors: impl IntoIterator<Item = Color>,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        Self {
            teams: team_colors
                .into_iter()
                .map(|color| TeamAssets::new(color, meshes, materials))
                .collect(),
        }
    }
}

/// Assets shared by all players on the same team.
pub struct TeamAssets {
    pub color: Color,
    pub mesh: Handle<Mesh>,
    pub size: Vec3,
    pub capsule_radius: f32,
    pub capsule_length: f32,
    pub out_of_play_material: Handle<StandardMaterial>,
}

impl TeamAssets {
    pub fn new(
        color: Color,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        // 1.8 meters tall.
        let height = 1.8;
        let capsule_radius = 0.18;
        let diam = 2.0 * capsule_radius;
        let capsule_length = height - diam;
        let size = Vec3::new(diam, height, diam);
        Self {
            color,
            size,
            capsule_radius,
            capsule_length,
            mesh: meshes.add(
                shape::Capsule {
                    radius: capsule_radius,
                    depth: capsule_length,
                    ..default()
                }
                .try_into()
                .unwrap(),
            ),
            out_of_play_material: materials.add(color.with_a(0.2).into()),
        }
    }
}
