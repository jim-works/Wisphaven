pub mod entity_loader;

pub use entity_loader::ChunkLoader;

use bevy::prelude::*;

use crate::{world::{LevelSystemSet, events::CreateLevelEvent, LevelLoadState, Level, chunk::ChunkCoord, settings::Settings}, physics::ChunkColliderGenerated};

use self::entity_loader::{DespawnChunkEvent, ChunkLoadingTimer};

pub struct ChunkLoaderPlugin;


impl Plugin for ChunkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((entity_loader::do_loading,entity_loader::unload_all).in_set(LevelSystemSet::Main))
            .add_system(entity_loader::despawn_chunks.in_set(LevelSystemSet::Despawn))
            .add_system(on_level_created.in_set(OnUpdate(LevelLoadState::Loading)))
            .add_system(finish_loading_trigger.in_set(OnUpdate(LevelLoadState::Loading)))
            .insert_resource(ChunkLoadingTimer {
                timer: Timer::from_seconds(0.1, TimerMode::Repeating)
            })
            .add_event::<DespawnChunkEvent>()
            .add_event::<LevelLoaded>();
    }
}

#[derive(Component)]
pub struct InitalLoader;

pub struct LevelLoaded;

pub fn on_level_created(
    mut reader: EventReader<CreateLevelEvent>,
    settings: Res<crate::settings::Settings>,
    mut commands: Commands
) {
    for _ in reader.iter() {
        commands.spawn((
            settings.init_loader.clone(),
            TransformBundle::from_transform(Transform::from_xyz(0.0,0.0,0.0)),
            InitalLoader
        ));
    }
}

pub fn finish_loading_trigger (
    mut next_state: ResMut<NextState<LevelLoadState>>,
    level: Res<Level>,
    init_loader: Query<(Entity, &ChunkLoader, &GlobalTransform), With<InitalLoader>>,
    loaded_chunk_query: Query<&ChunkColliderGenerated>,
    mut commands: Commands
) {
    info!("Checking if level is loaded...");
    for (_, loader, tf) in init_loader.iter() {
        loader.for_each_chunk(|coord| {
            if let Some(chunk_ref) = level.get_chunk(coord+tf.translation().into()) {
                match chunk_ref.value() {
                    crate::world::chunk::ChunkType::Ungenerated(_) => return,
                    crate::world::chunk::ChunkType::Full(chunk) => if !loaded_chunk_query.contains(chunk.entity) {
                        return;
                    }
                }
            }
        });
    }
    info!("Finished loading the level!");
    //if we make it here, all chunks are loaded
    for (entity, _, _) in init_loader.iter() {
        commands.entity(entity).despawn();
    }
    next_state.set(LevelLoadState::Loaded);
}