pub mod entity_loader;

pub use entity_loader::ChunkLoader;

use bevy::prelude::*;

use crate::{world::{LevelSystemSet, LevelLoadState, Level, settings::Settings}, actors::spawn_local_player, util::LocalRepeatingTimer, physics::ChunkColliderGenerated};

use self::entity_loader::{DespawnChunkEvent, ChunkLoadingTimer};

pub struct ChunkLoaderPlugin;


impl Plugin for ChunkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, entity_loader::do_loading.in_set(LevelSystemSet::LoadingAndMain))
            .add_systems(PostUpdate, entity_loader::despawn_chunks.in_set(LevelSystemSet::Despawn))
            .add_systems(Update, finish_loading_trigger.run_if(in_state(LevelLoadState::Loading)))
            .add_systems(OnEnter(LevelLoadState::Loading), on_load_level)
            //I'm not sure this .after is necessary, since both systems should run the same frame and commands may be applied at the end of that frame
            .add_systems(OnEnter(LevelLoadState::Loaded), despawn_initial_loader.after(spawn_local_player))
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
    commands.spawn((SpatialBundle::from_transform(spawn_point),InitialLoader,settings.init_loader.clone()));
}

pub fn finish_loading_trigger (
    mut next_state: ResMut<NextState<LevelLoadState>>,
    level: Res<Level>,
    //check loading every 100 ms
    mut timer: Local<LocalRepeatingTimer<100>>,
    time: Res<Time>,
    init_loader: Query<(Entity, &ChunkLoader, &GlobalTransform), With<InitialLoader>>,
   loaded_chunk_query: Query<&ChunkColliderGenerated>
) {
    timer.tick(time.delta());
    if !timer.just_finished() {
        return;
    }
    let mut loaded = 0;
    let mut target = 0;
    for (_, loader, tf) in init_loader.iter() {
        loader.for_each_center_chunk(|coord| {
            target += 1;
            if let Some(chunk_ref) = level.get_chunk(coord+tf.translation().into()) {
                match chunk_ref.value() {
                    crate::world::chunk::ChunkType::Full(chunk) => if loaded_chunk_query.contains(chunk.entity) {
                        loaded += 1;
                    },
                    _ => {}
                    
                }
            }
        });
    }
    //we not all chunks will be loaded (due to some boundary chunks waiting during world generation), so approximate with target/2
    if loaded >= target && !init_loader.is_empty() {
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