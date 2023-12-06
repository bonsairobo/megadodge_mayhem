use crate::{
    aabb::Aabb2,
    player::{KnockedOut, Player, PlayerAssets, PlayerBall, PlayerBundle},
};
use bevy::prelude::*;
use rand::Rng;

#[derive(Component)]
pub struct Squad {
    pub squad: u8,
}

impl Squad {
    pub fn new(squad: u8) -> Self {
        Self { squad }
    }

    pub fn spawn(
        commands: &mut Commands,
        assets: &PlayerAssets,
        team: u8,
        squad: u8,
        aabb: Aabb2,
        n_players: u32,
    ) -> Entity {
        assert!(n_players > 0);

        let y = 0.5 * assets.size.y;

        let mut rng = rand::thread_rng();
        for _ in 0..n_players {
            let x = rng.gen_range(aabb.min.x..aabb.max.x);
            let z = rng.gen_range(aabb.min.y..aabb.max.y);
            commands.spawn(PlayerBundle::new(assets, team, squad, Vec3::new(x, y, z)));
        }

        let ai_pos = aabb.center();
        commands
            .spawn(SquadAiBundle::new(
                squad,
                Vec3::new(ai_pos.x, 0.0, ai_pos.y),
            ))
            .id()
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
            throw_rate: 1.0,
            throw_min_balls: 1,
            throw_y_vel: 4.0,
            auto_throw: true,
            stats: default(),
        }
    }
}

pub type Probability = f32;

pub struct PlayerStats {
    // TODO: use this
    pub dodge_speed: f32,
    pub run_speed: f32,
    // TODO: use this
    pub throw_accuracy: f32,
    // TODO: calculate this from throw velocity
    pub throw_distance: f32,
    pub throw_speed: f32,

    // TODO: use this
    pub p_block: Probability,
    // TODO: use this
    pub p_catch: Probability,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            dodge_speed: 1.0,
            run_speed: 7.0,
            throw_accuracy: 0.8,
            throw_distance: 10.0,
            throw_speed: 20.0,
            p_block: 0.7,
            p_catch: 0.2,
        }
    }
}

#[derive(Resource)]
pub struct SquadStates {
    squads: Vec<SquadState>,
}

impl SquadStates {
    pub fn new(squads: impl IntoIterator<Item = u32>) -> Self {
        Self {
            squads: squads.into_iter().map(SquadState::new).collect(),
        }
    }

    #[allow(clippy::complexity)]
    pub fn update(
        mut states: ResMut<Self>,
        players: Query<(&Squad, &PlayerBall), (With<Player>, Without<KnockedOut>)>,
    ) {
        for state in &mut states.squads {
            state.clear();
        }
        for (squad, player_ball) in &players {
            let state = &mut states.squads[squad.squad as usize];
            state.num_players += 1;
            if player_ball.holding_ball {
                state.num_holding_balls += 1;
            }
        }
    }
}

pub struct SquadState {
    pub num_players: u32,
    pub num_holding_balls: u32,
}

impl SquadState {
    fn new(num_players: u32) -> Self {
        Self {
            num_players,
            num_holding_balls: 0,
        }
    }

    fn clear(&mut self) {
        self.num_players = 0;
        self.num_holding_balls = 0;
    }
}

#[derive(Component)]
pub struct SquadAi;

#[derive(Bundle)]
pub struct SquadAiBundle {
    pub ai: SquadAi,
    pub squad: Squad,
    pub transform: TransformBundle,
}

impl SquadAiBundle {
    fn new(squad: u8, position: Vec3) -> Self {
        Self {
            ai: SquadAi,
            squad: Squad::new(squad),
            transform: TransformBundle {
                local: Transform::from_translation(position),
                ..default()
            },
        }
    }
}
