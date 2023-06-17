pub mod entity_loader;

pub use entity_loader::ChunkLoader;

use bevy::prelude::*;

use crate::{world::{LevelSystemSet, events::CreateLevelEvent, LevelLoadState, Level, chunk::ChunkCoord, settings::Settings}, physics::ChunkColliderGenerated, actors::spawn_local_player};

use self::entity_loader::{DespawnChunkEvent, ChunkLoadingTimer};

pub struct ChunkLoaderPlugin;


impl Plugin for ChunkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((entity_loader::do_loading,entity_loader::unload_all).in_set(LevelSystemSet::LoadingAndMain))
            .add_system(entity_loader::despawn_chunks.in_set(LevelSystemSet::Despawn))
            .add_system(finish_loading_trigger.in_set(OnUpdate(LevelLoadState::Loading)))
            .add_system(on_load_level.in_schedule(OnEnter(LevelLoadState::Loading)))
            //I'm not sure this .after is necessary, since both systems should run the same frame and commands may be applied at the end of that frame
            .add_system(despawn_initial_loader.in_schedule(OnEnter(LevelLoadState::Loaded)).after(spawn_local_player))
            .insert_resource(ChunkLoadingTimer {
                timer: Timer::from_seconds(0.1, TimerMode::Repeating)
            })
            .add_event::<DespawnChunkEvent>()
        ;
    }
}

#[derive(Component)]
pub struct InitialLoader;

pub fn on_load_level (
    mut commands: Commands,
    settings: Res<Settings>,
    level: Res<Level>
) {
    let spawn_point = Transform::from_translation(level.spawn_point);
    info!("creating inital loader at {:?} loader: {:?}", spawn_point, settings.init_loader);
    commands.spawn((TransformBundle::from_transform(spawn_point),InitialLoader,settings.init_loader.clone()));
}

pub fn finish_loading_trigger (
    mut next_state: ResMut<NextState<LevelLoadState>>,
    level: Res<Level>,
    init_loader: Query<(Entity, &ChunkLoader, &GlobalTransform), With<InitialLoader>>,
//    loaded_chunk_query: Query<&ChunkColliderGenerated>
) {
    let mut loaded = 0;
    let mut target = 0;
    for (_, loader, tf) in init_loader.iter() {
        loader.for_each_chunk(|coord| {
            target += 1;
            if let Some(chunk_ref) = level.get_chunk(coord+tf.translation().into()) {
                match chunk_ref.value() {
                    crate::world::chunk::ChunkType::Ungenerated(_) => return,
                    crate::world::chunk::ChunkType::Full(_) => loaded += 1
                    // crate::world::chunk::ChunkType::Full(chunk) => if loaded_chunk_query.contains(chunk.entity) {
                    //     loaded += 1;
                    // }
                }
            }
        });
    }
    if loaded == target && !init_loader.is_empty() {
        info!("Finished loading the level! {}/{} Chunks loaded!", loaded, target);
        next_state.set(LevelLoadState::Loaded);
    } else {
        info!("Loaded {} out of {} chunks", loaded, target);
    }
}

pub fn despawn_initial_loader(
    init_loader: Query<Entity, With<InitialLoader>>,
    mut commands: Commands,
) {
    for entity in init_loader.iter() {
        commands.entity(entity).despawn_recursive();
    }
}