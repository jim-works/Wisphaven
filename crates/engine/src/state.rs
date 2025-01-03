use bevy::prelude::*;

use crate::{
    actors::{
        world_anchor::{ActiveWorldAnchor, WorldAnchorHasSpawned},
        Player, RespawningPlayer,
    },
    world::LevelSystemSet,
    GameState,
};

pub(crate) struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            detect_game_over
                .in_set(LevelSystemSet::PostTick)
                .run_if(resource_exists::<WorldAnchorHasSpawned>)
                .run_if(not(resource_exists::<ActiveWorldAnchor>)),
        )
        .init_state::<GameState>()
        .enable_state_scoped_entities::<GameState>();
    }
}

fn detect_game_over(
    mut next_state: ResMut<NextState<GameState>>,
    player_query: Query<(), With<Player>>,
    respawning_player: Res<RespawningPlayer>,
) {
    if player_query.is_empty() && respawning_player.0.is_none() {
        next_state.set(GameState::GameOver);
        info!("Game Over!");
    }
}
