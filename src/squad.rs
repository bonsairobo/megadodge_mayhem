use crate::{
    collision,
    geometry::Aabb2,
    opponent_ai::Bot,
    parameters::{
        BLOOM_INTENSITY, SQUAD_AI_COLLIDER_HEIGHT, SQUAD_AI_COLLIDER_RADIUS, SQUAD_CLUSTER_DENSITY,
    },
    player::{KnockedOut, Player, PlayerBall, PlayerBundle},
    team::{AllTeamAssets, Team, TeamAssets},
};
use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::{Collider, CollisionGroups, Group, QueryFilter, RapierContext};
use rand::Rng;
use std::ops::Range;

#[derive(Clone, Copy, Component)]
pub struct Squad {
    pub squad: u8,
}

impl Squad {
    pub fn new(squad: u8) -> Self {
        Self { squad }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        commands: &mut Commands,
        team_assets: &TeamAssets,
        squad_assets: &SquadAssets,
        team: Team,
        squad: u8,
        aabb: Aabb2,
        leader_pos: Vec2,
        n_players: u32,
    ) -> Entity {
        assert!(n_players > 0);

        let y = 0.5 * team_assets.size.y;

        let mut rng = rand::thread_rng();
        for _ in 0..n_players {
            let x = rng.gen_range(aabb.min.x..aabb.max.x);
            let z = rng.gen_range(aabb.min.y..aabb.max.y);
            commands.spawn(PlayerBundle::new(
                team_assets,
                squad_assets,
                team,
                squad,
                Vec3::new(x, y, z),
            ));
        }

        let mut commands = commands.spawn(SquadAiBundle::new(
            team,
            squad,
            Vec3::new(leader_pos.x, 0.0, leader_pos.y),
        ));
        if team.is_human() {
            commands.insert(SquadAiPickableBundle::new(squad));
        } else {
            commands.insert(Bot);
        }

        commands.id()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_in_line(
        commands: &mut Commands,
        team_assets: &TeamAssets,
        squad_assets: &AllSquadAssets,
        team: Team,
        squads: Range<u8>,
        aabb: Aabb2,
        players_per_squad: u32,
        squad_ai_entities: &mut Vec<Entity>,
    ) {
        let n_squads = squads.len();
        let dx_squad = (aabb.max.x - aabb.min.x) / n_squads as f32;
        let c = aabb.center();
        let halfway_to_center_line = c.y / 2.0;
        let start = Vec2::new(aabb.min.x + 0.5 * dx_squad, halfway_to_center_line);
        squad_ai_entities.extend(squads.enumerate().map(|(i, squad)| {
            let leader_pos = start + dx_squad * Vec2::X * i as f32;
            Squad::spawn(
                commands,
                team_assets,
                &squad_assets.squads[squad as usize],
                team,
                squad,
                aabb,
                leader_pos,
                players_per_squad,
            )
        }));
    }
}

#[derive(Resource)]
pub struct AllSquadAssets {
    pub squads: Vec<SquadAssets>,
}

impl AllSquadAssets {
    pub fn new(
        squad_colors: impl IntoIterator<Item = Color>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        Self {
            squads: squad_colors
                .into_iter()
                .map(|color| SquadAssets::new(color, materials))
                .collect(),
        }
    }
}

pub struct SquadAssets {
    pub in_play_material: Handle<StandardMaterial>,
}

impl SquadAssets {
    pub fn new(color: Color, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            in_play_material: materials.add(color.into()),
        }
    }
}

#[derive(Resource)]
pub struct SquadBehaviors {
    pub squads: Vec<SquadBehavior>,
}

impl SquadBehaviors {
    pub fn new(leaders: impl IntoIterator<Item = Entity>) -> Self {
        Self {
            squads: leaders.into_iter().map(SquadBehavior::new).collect(),
        }
    }

    pub fn set_leader_position(
        event: Listener<Pointer<Click>>,
        mut behaviors: ResMut<Self>,
        states: Res<SquadStates>,
    ) {
        if event.button != PointerButton::Primary {
            return;
        }
        let (Some(selected_squad), Some(position)) = (states.selected, event.hit.position) else {
            return;
        };

        let behavior = &mut behaviors.squads[selected_squad as usize];
        behavior.leader_position = Some(position.xz());
    }
}

pub struct SquadBehavior {
    /// SquadAi at the center of the squad formation.
    ///
    /// This entity may also be used to reduce the number of spatial queries,
    /// assuming the squad will stay close to them.
    pub leader: Entity,
    /// A manually configured leader position. If None, leader will decide where
    /// to go.
    pub leader_position: Option<Vec2>,
    /// Players per square meter.
    pub cluster_density: f32,
    /// Balls per second.
    pub throw_rate: f32,
    /// Minimum number of balls to throw at a time.
    pub throw_min_balls: u32,
    /// How much Y velocity (loft) a throw should have.
    pub throw_y_vel: f32,
    /// When true, player AIs will decide when to throw. When false, the human
    /// player must provide input to make then throw.
    pub auto_throw: bool,
    pub stats: PlayerStats,
}

impl SquadBehavior {
    pub fn new(leader: Entity) -> Self {
        Self {
            leader,
            leader_position: None,
            cluster_density: SQUAD_CLUSTER_DENSITY,
            throw_rate: 1.0,
            throw_min_balls: 1,
            throw_y_vel: 4.0,
            auto_throw: true,
            stats: default(),
        }
    }
}

pub struct PlayerStats {
    pub run_speed: f32,
    pub throw_distance: f32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            run_speed: 10.0,
            throw_distance: 15.0,
        }
    }
}

#[derive(Resource)]
pub struct SquadStates {
    pub selected: Option<u8>,
    pub squads: Vec<SquadState>,
}

impl SquadStates {
    pub fn new(squads: impl IntoIterator<Item = u32>) -> Self {
        Self {
            selected: None,
            squads: squads.into_iter().map(SquadState::new).collect(),
        }
    }

    #[allow(clippy::complexity)]
    pub fn update(
        mut states: ResMut<Self>,
        behaviors: Res<SquadBehaviors>,
        mut squad_ai_colliders: Query<&mut Collider, With<SquadAi>>,
        squad_ais: Query<&GlobalTransform, With<SquadAi>>,
        players: Query<
            (&Squad, &GlobalTransform, &PlayerBall),
            (With<Player>, Without<KnockedOut>),
        >,
    ) {
        for state in &mut states.squads {
            // Reset counters that we use below.
            state.num_players = 0;
            state.num_players_in_cluster = 0;
            state.num_holding_balls = 0;
            state.center_of_mass = Vec3::ZERO;
        }

        // Squad accounting.
        for (squad, tfm, player_ball) in &players {
            let state = &mut states.squads[squad.squad as usize];
            state.num_players += 1;
            if player_ball.holding_ball {
                state.num_holding_balls += 1;
            }

            state.center_of_mass += tfm.translation();

            let behavior = &behaviors.squads[squad.squad as usize];
            let Ok(leader_tfm) = squad_ais.get(behavior.leader) else {
                continue;
            };

            let dist_to_leader = leader_tfm.translation().distance(tfm.translation());
            if dist_to_leader < state.cluster_radius {
                state.num_players_in_cluster += 1;
            }
        }
        for state in &mut states.squads {
            state.center_of_mass /= state.num_players as f32;
        }

        for ((squad, state), behavior) in (0..).zip(&mut states.squads).zip(&behaviors.squads) {
            state.set_cluster_radius(behavior.cluster_density);

            // Update squad AI colliders.
            let Ok(mut collider) = squad_ai_colliders.get_mut(behavior.leader) else {
                continue;
            };
            *collider = Collider::cylinder(squad_collider_height(squad), SQUAD_AI_COLLIDER_RADIUS);
        }
    }
}

#[derive(Default)]
pub struct SquadState {
    pub num_players: u32,
    pub num_holding_balls: u32,
    pub num_players_in_cluster: u32,
    pub center_of_mass: Vec3,
    pub cluster_radius: f32,
    pub throw_target: Option<Vec3>,
}

impl SquadState {
    fn new(num_players: u32) -> Self {
        Self {
            num_players,
            ..default()
        }
    }

    pub fn ball_percent(&self) -> u32 {
        (100 * self.num_holding_balls) / self.num_players.max(1)
    }

    pub fn cluster_percent(&self) -> u32 {
        (100 * self.num_players_in_cluster) / self.num_players.max(1)
    }

    fn set_cluster_radius(&mut self, density: f32) {
        // density = players / area
        // area = players / density
        // radius^2 = (players / density) / PI
        self.cluster_radius = ((self.num_players as f32 / density) / std::f32::consts::PI).sqrt();
    }
}

#[derive(Component)]
pub struct SquadAi;

impl SquadAi {
    pub fn select_squad(
        event: Listener<Pointer<Click>>,
        mut states: ResMut<SquadStates>,
        all_squad_assets: Res<AllSquadAssets>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        squad_ais: Query<&Squad, With<Self>>,
    ) {
        if event.button != PointerButton::Primary {
            return;
        }
        let Ok(selected_squad) = squad_ais.get(event.target) else {
            return;
        };
        if let Some(old_selected) = states.selected {
            if old_selected != selected_squad.squad {
                let squad_assets = &all_squad_assets.squads[old_selected as usize];
                if let Some(material) = materials.get_mut(&squad_assets.in_play_material) {
                    material.emissive = Color::BLACK;
                }
            }
        }
        states.selected = Some(selected_squad.squad);
    }

    pub fn highlight_squad(
        event: Listener<Pointer<Over>>,
        all_team_assets: Res<AllTeamAssets>,
        all_squad_assets: Res<AllSquadAssets>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        squad_ais: Query<(&Team, &Squad), With<Self>>,
    ) {
        let Ok((team, squad)) = squad_ais.get(event.target) else {
            return;
        };

        let team_assets = &all_team_assets.teams[team.team() as usize];
        let squad_assets = &all_squad_assets.squads[squad.squad as usize];
        let Some(material) = materials.get_mut(&squad_assets.in_play_material) else {
            return;
        };

        material.emissive = team_assets.color * BLOOM_INTENSITY;
    }

    pub fn unhighlight_squad(
        event: Listener<Pointer<Out>>,
        states: Res<SquadStates>,
        all_squad_assets: Res<AllSquadAssets>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        squad_ais: Query<&Squad, With<Self>>,
    ) {
        let Ok(squad) = squad_ais.get(event.target) else {
            return;
        };

        if states.selected == Some(squad.squad) {
            // Keep the selection highlighted.
            return;
        }

        let squad_assets = &all_squad_assets.squads[squad.squad as usize];
        let Some(material) = materials.get_mut(&squad_assets.in_play_material) else {
            return;
        };

        material.emissive = Color::BLACK;
    }

    pub fn move_to_requested_positions(
        behaviors: Res<SquadBehaviors>,
        mut squad_ais: Query<(&Squad, &mut Transform), With<Self>>,
    ) {
        for (squad, mut tfm) in &mut squad_ais {
            let behavior = &behaviors.squads[squad.squad as usize];
            if let Some(requested_pos) = behavior.leader_position {
                tfm.translation = Vec3::new(requested_pos.x, 0.0, requested_pos.y);
            }
        }
    }

    pub fn find_target_enemy(
        rapier_context: Res<RapierContext>,
        mut states: ResMut<SquadStates>,
        mut squad_ais: Query<(&Squad, &Team), With<Self>>,
        teams: Query<&Team>,
        transforms: Query<&GlobalTransform>,
    ) {
        for (squad, squad_team) in &mut squad_ais {
            let state = &mut states.squads[squad.squad as usize];

            state.throw_target = None;

            let entity_on_enemy_team = |entity| {
                teams
                    .get(entity)
                    .map(|team| team.team() != squad_team.team())
                    .unwrap_or_default()
            };
            let select_enemy_players = QueryFilter::new()
                .groups(CollisionGroups::new(
                    collision::groups::QUERY,
                    collision::groups::PLAYER,
                ))
                .predicate(&entity_on_enemy_team);

            // Identify the closest target.
            state.throw_target = rapier_context
                .project_point(state.center_of_mass, true, select_enemy_players)
                .and_then(|(nearest_player_entity, _)| {
                    transforms
                        .get(nearest_player_entity)
                        .ok()
                        .map(GlobalTransform::translation)
                });
        }
    }
}

#[derive(Bundle)]
pub struct SquadAiBundle {
    pub ai: SquadAi,
    pub team: Team,
    pub squad: Squad,
    pub transform: TransformBundle,
    pub collision_groups: CollisionGroups,
}

impl SquadAiBundle {
    fn new(team: Team, squad: u8, position: Vec3) -> Self {
        Self {
            ai: SquadAi,
            team,
            squad: Squad::new(squad),
            transform: TransformBundle {
                local: Transform::from_translation(position),
                ..default()
            },
            // I'm a little surprised that picking still works with these
            // collision filters, but I'm not complaining.
            collision_groups: CollisionGroups::new(Group::NONE, Group::NONE),
        }
    }
}

#[derive(Bundle)]
pub struct SquadAiPickableBundle {
    pub rapier_pickable: RapierPickable,
    pub pickable: Pickable,
    pub collider: Collider,
    pub on_click: On<Pointer<Click>>,
    pub on_over: On<Pointer<Over>>,
    pub on_out: On<Pointer<Out>>,
}

impl SquadAiPickableBundle {
    pub fn new(squad: u8) -> Self {
        Self {
            rapier_pickable: RapierPickable,
            pickable: Pickable {
                // BUG: not working?
                should_block_lower: false,
                should_emit_events: true,
            },
            // The radius will update as the squad cluster radius changes.
            collider: Collider::cylinder(squad_collider_height(squad), 1.0),
            on_click: On::<Pointer<Click>>::run(SquadAi::select_squad),
            on_over: On::<Pointer<Over>>::run(SquadAi::highlight_squad),
            on_out: On::<Pointer<Out>>::run(SquadAi::unhighlight_squad),
        }
    }
}

fn squad_collider_height(squad: u8) -> f32 {
    // HACK: we need some way to prioritize picking when squads overlap.
    // It's pretty easy to just give them colliders of different heights.
    f32::from(squad + 1) * SQUAD_AI_COLLIDER_HEIGHT
}
