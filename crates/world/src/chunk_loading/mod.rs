pub mod entity_loader;

pub use entity_loader::ChunkLoader;

use bevy::prelude::*;

use crate::{level::Level, settings::Settings};
use interfaces::scheduling::*;
use util::LocalRepeatingTimer;

use self::entity_loader::{ChunkLoadingTimer, DespawnChunkEvent};

pub struct ChunkLoaderPlugin;

impl Plugin for ChunkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            entity_loader::do_loading.in_set(LevelSystemSet::LoadingAndMain),
        )
        .add_systems(
            PostUpdate,
            entity_loader::despawn_chunks.in_set(LevelSystemSet::Despawn),
        )
        .add_systems(
            Update,
            (
                finish_loading_trigger.run_if(
                    in_state(LevelLoadState::Loading).and(not(in_state(NetworkType::Client))),
                ),
                //todo: this is temporary
                (|mut next_state: ResMut<NextState<LevelLoadState>>| {
                    next_state.set(LevelLoadState::Loaded);
                })
                .run_if(in_state(LevelLoadState::Loading).and(in_state(ClientState::Ready))),
            ),
        )
        .add_systems(OnEnter(LevelLoadState::Loading), on_load_level)
        .add_systems(OnEnter(LevelLoadState::Loaded), despawn_initial_loader)
        .insert_resource(ChunkLoadingTimer {
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        })
        .add_event::<DespawnChunkEvent>();
    }
}

#[derive(Component)]
pub struct InitialLoader;

pub fn on_load_level(mut commands: Commands, settings: Res<Settings>, level: Res<Level>) {
    let spawn_point = Transform::from_translation(level.get_initial_spawn_point());
    info!(
        "creating inital loader at {:?} loader: {:?}",
        spawn_point, settings.init_loader
    );
    commands.spawn((
        StateScoped(LevelLoadState::Loading),
        spawn_point,
        InitialLoader,
        settings.init_loader.clone(),
    ));
}

pub fn finish_loading_trigger(
    mut next_state: ResMut<NextState<LevelLoadState>>,
    level: Res<Level>,
    //check loading every 100 ms
    mut timer: Local<LocalRepeatingTimer<100>>,
    time: Res<Time>,
    init_loader: Query<(Entity, &ChunkLoader, &GlobalTransform), With<InitialLoader>>,
) {
    timer.tick(time.delta());
    if !timer.just_finished() {
        return;
    }
    if init_loader.is_empty() {
        warn!("no init loader");
    } else {
        info!("yes init loader");
    }
    let mut loaded = 0;
    let mut target = 0;
    for (_, loader, tf) in init_loader.iter() {
        loader.for_each_center_chunk(|coord| {
            target += 1;
            if let Some(chunk_ref) = level.get_chunk(coord + tf.translation().into()) {
                if let crate::chunk::ChunkType::Full(_) = chunk_ref.value() {
                    loaded += 1;
                }
            }
        });
    }
    if loaded >= target && !init_loader.is_empty() {
        info!(
            "Finished loading the level! {}/{} Chunks loaded!",
            loaded, target
        );
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
