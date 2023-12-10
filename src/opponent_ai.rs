use crate::{
    boundaries::Boundaries,
    geometry::{Circle, Ray2},
    gym::GymParams,
    settings::{GameConfig, GameMode},
    squad::{AllSquadAssets, Squad, SquadAi, SquadBehaviors, SquadState, SquadStates},
    team::{AllTeamAssets, Team},
};
use bevy::{
    ecs::system::{Command, RunSystemOnce},
    prelude::*,
};
use rand::Rng;

#[derive(Component)]
pub struct Bot;

/// Sets leader tokens for squads to follow.
#[allow(clippy::complexity)]
pub fn control_bot_team(
    mut commands: Commands,
    config: Res<GameConfig>,
    bounds: Res<Boundaries>,
    mut behaviors: ResMut<SquadBehaviors>,
    states: Res<SquadStates>,
    bot_squad_ais: Query<(&Team, &Squad, &GlobalTransform), (With<Bot>, With<SquadAi>)>,
    all_squad_ais: Query<(&Team, &Squad), With<SquadAi>>,
) {
    for (team, squad, tfm) in &bot_squad_ais {
        let state = &states.squads[squad.squad as usize];
        if state.num_players == 0 {
            if config.mode == GameMode::Survival {
                commands.add(RespawnSquad::new(*team, *squad));
            }
            continue;
        }

        let pos = state.center_of_mass;

        let threats =
            ThreatLevels::assess(&behaviors, &states, &all_squad_ais, team, squad, state, tfm);

        if let Some((scary, scary_pos)) = threats.scary {
            let enemy_state = &states.squads[scary as usize];
            run_from_enemy(
                &bounds,
                &mut behaviors,
                squad,
                state,
                pos,
                scary,
                enemy_state,
                scary_pos,
            );
        } else {
            // We aren't scared. Let's go on the attack.
            if let Some((_target_squad, target_squad_pos)) = threats.vulnerable {
                let behavior = &mut behaviors.squads[squad.squad as usize];
                let target_dist = target_squad_pos.distance(pos);
                let throw_dist = (behavior.stats.throw_distance - state.cluster_radius).max(0.0);
                let ratio = throw_dist / target_dist;
                let throw_pos = ratio * pos + (1.0 - ratio) * target_squad_pos;
                behavior.leader_position = Some(throw_pos.xz());
            } else {
                // TODO
                // let ball_pos = find_balls();
                // if let Some(ball_pos) = ball_pos {
                //     let behavior = &mut behaviors.squads[squad.squad as usize];
                //     behavior.leader_position = Some(ball_pos.xz());
                // }
            }
        }
    }
}

#[derive(Default)]
struct ThreatLevels {
    /// Most threatening squad and location.
    scary: Option<(u8, Vec3)>,
    /// Least threatening squad and location.
    vulnerable: Option<(u8, Vec3)>,
}

impl ThreatLevels {
    fn assess(
        behaviors: &SquadBehaviors,
        states: &SquadStates,
        all_squad_ais: &Query<(&Team, &Squad), With<SquadAi>>,
        team: &Team,
        squad: &Squad,
        state: &SquadState,
        tfm: &GlobalTransform,
    ) -> Self {
        let max_ball_ratio = 2.0;

        let mut threat_levels = Self::default();
        let mut max_scary = 0.0;
        let mut max_vuln = 0.0;
        for (enemy_team, enemy_squad) in all_squad_ais {
            if enemy_team.team() == team.team() {
                continue;
            }

            let enemy_state = &states.squads[enemy_squad.squad as usize];
            if enemy_state.num_players == 0 {
                continue;
            }

            let ball_ratio = if enemy_state.num_holding_balls == 0 {
                max_ball_ratio
            } else {
                state.num_holding_balls as f32 / enemy_state.num_holding_balls as f32
            };

            let enemy_dist = enemy_state.center_of_mass.distance(tfm.translation());

            let ball_factor = 0.5;
            let dist_factor = 0.5;

            if ball_ratio >= 1.0 {
                // We're not scared of them.

                // This becomes meaningless when there is a large imbalance.
                let ball_ratio = ball_ratio.min(max_ball_ratio);

                let behavior = &behaviors.squads[squad.squad as usize];
                let throw_dist = behavior.stats.throw_distance;
                let dist_ratio = throw_dist / enemy_dist.max(0.001);

                let vuln_metric = ball_factor * ball_ratio + dist_factor * dist_ratio;
                if vuln_metric > max_vuln {
                    threat_levels.vulnerable =
                        Some((enemy_squad.squad, enemy_state.center_of_mass));
                    max_vuln = vuln_metric;
                }
            } else {
                // We're scared of them.

                let ball_ratio = ball_ratio.recip();
                // This becomes meaningless when there is a large imbalance.
                let ball_ratio = ball_ratio.min(max_ball_ratio);

                let enemy_behavior = &behaviors.squads[enemy_squad.squad as usize];
                let enemy_throw_dist = enemy_behavior.stats.throw_distance;
                let dist_ratio = enemy_throw_dist / enemy_dist.max(0.001);

                let scary_metric = ball_factor * ball_ratio + dist_factor * dist_ratio;
                if scary_metric > max_scary {
                    threat_levels.scary = Some((enemy_squad.squad, enemy_state.center_of_mass));
                    max_scary = scary_metric;
                }
            }
        }
        threat_levels
    }
}

#[allow(clippy::too_many_arguments)]
fn run_from_enemy(
    bounds: &Boundaries,
    behaviors: &mut SquadBehaviors,
    squad: &Squad,
    state: &SquadState,
    pos: Vec3,
    enemy_squad: u8,
    enemy_state: &SquadState,
    enemy_pos: Vec3,
) {
    let enemy_behavior = &behaviors.squads[enemy_squad as usize];
    let safety_margin = 2.0;
    let safe_dist = safety_margin * (state.cluster_radius + enemy_behavior.stats.throw_distance);
    let current_dist = pos.distance(enemy_pos);
    if current_dist > safe_dist {
        return;
    }

    // Simple Pathfinding
    //
    // Cast rays in 8 directions. Choose the direction that doesn't
    // collide with the enemy's cluster while also allowing us to move a
    // safe distance away. When multiple options are available, choose
    // the one which maximizes distance from the enemy.
    let max_plan = 10.0;
    let breathing_room = state.cluster_radius;
    let bounds_aabb = bounds.aabb2();
    let scary_circle = Circle::new(enemy_pos.xz(), enemy_state.cluster_radius);
    let sqrt2 = std::f32::consts::SQRT_2;
    let ray_dirs = [
        Vec2::new(-sqrt2, -sqrt2),
        Vec2::new(-1.0, 0.0),
        Vec2::new(-sqrt2, sqrt2),
        Vec2::new(0.0, -1.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(sqrt2, -sqrt2),
        Vec2::new(1.0, 0.0),
        Vec2::new(sqrt2, sqrt2),
    ];
    let mut best_dir = None;
    let mut max_dist_from_enemy = 0.0;
    for ray_dir in ray_dirs {
        let ray = Ray2::new(pos.xz(), ray_dir);
        if scary_circle.cast_ray(ray).is_some() {
            // Don't go in directions that intersect the scary enemy.
            continue;
        }
        let Some(toi) = bounds_aabb.cast_ray(false, f32::INFINITY, ray) else {
            // Should never happen, but why tempt fate by unwrapping?
            continue;
        };

        if toi < breathing_room {
            continue;
        }

        // Can we get to a safe distance along this direction?
        let end = ray.at(toi.max(max_plan));
        let end_dist_from_enemy = end.distance(enemy_pos.xz());
        if end_dist_from_enemy < safe_dist {
            continue;
        }

        if end_dist_from_enemy > max_dist_from_enemy {
            best_dir = Some(ray_dir);
            max_dist_from_enemy = end_dist_from_enemy;
        }
    }

    // Set leader position.
    if let Some(move_dir) = best_dir {
        let avoid_vec = move_dir * max_plan;
        let behavior = &mut behaviors.squads[squad.squad as usize];
        behavior.leader_position = Some(pos.xz() + avoid_vec);
    }
}

struct RespawnSquad {
    team: Team,
    squad: Squad,
}

impl RespawnSquad {
    fn new(team: Team, squad: Squad) -> Self {
        Self { team, squad }
    }
}

impl Command for RespawnSquad {
    fn apply(self, world: &mut World) {
        world.run_system_once_with(self, spawn_squad);
    }
}

// Move the leader token back into the spawn area and respawn all players.
fn spawn_squad(
    respawn: In<RespawnSquad>,
    mut commands: Commands,
    config: Res<GameConfig>,
    gym_params: Res<GymParams>,
    mut behaviors: ResMut<SquadBehaviors>,
    team_assets: Res<AllTeamAssets>,
    squad_assets: Res<AllSquadAssets>,
) {
    let In(RespawnSquad { team, squad }) = respawn;

    let spawn_aabbs = gym_params.player_spawn_aabbs();
    let aabb = spawn_aabbs[team.team() as usize];

    let behavior = &mut behaviors.squads[squad.squad as usize];

    commands.entity(behavior.leader).despawn_recursive();

    let mut rng = rand::thread_rng();
    let x = rng.gen_range(aabb.min.x..aabb.max.x);
    let z = rng.gen_range(aabb.min.y..aabb.max.y);
    let leader_pos = Vec2::new(x, z);
    behavior.leader = Squad::spawn(
        &mut commands,
        &team_assets.teams[team.team() as usize],
        &squad_assets.squads[squad.squad as usize],
        team,
        squad.squad,
        aabb,
        leader_pos,
        config.players_per_squad,
    );
}
