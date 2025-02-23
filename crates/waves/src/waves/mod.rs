use std::{f32::consts::PI, sync::Arc, time::Duration};

use bevy::prelude::*;
use interfaces::scheduling::LevelSystemSet;
use itertools::Itertools;
use rand::{thread_rng, RngCore};

use engine::actors::world_anchor::ActiveWorldAnchor;
use world::{
    atmosphere::{Calendar, NightStartedEvent},
    block::{BlockCoord, BlockType},
    level::Level,
};

use spawns::DefaultSpawn;
use util::{
    get_wrapping,
    iterators::{Volume, VolumeContainer},
};

pub mod spawns;

pub struct WavesPlugin;

impl Plugin for WavesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (trigger_assault, spawn_wave)
                .chain()
                .in_set(LevelSystemSet::PreTick),
        )
        .add_systems(
            FixedUpdate,
            despawn_assaults.in_set(LevelSystemSet::PostTick),
        )
        .add_event::<WaveStartedEvent>();
    }
}

#[derive(Event)]
pub struct WaveStartedEvent(pub Entity, pub usize);

#[derive(Component, Default)]
pub struct Assault {
    pub waves: Vec<WaveInfo>,
    //create by calling .compile(), sorted in descending time order (so you can pop)
    pub compiled: Vec<CompiledSpawn>,
    pub possible_spawns: Vec<SpawnableEntity>,
    pub spawn_points: Vec<SpawnPoint>,
}

#[derive(Component, Default)]
pub struct ActiveAssault;

impl Assault {
    //assumes possible_spawns is sorted in ascending order
    //returns spawn with lowest strength >= min_strength, or the spawn with the highest strength if none are possible
    pub fn get_spawn_idx(&self, min_strength: f32) -> Option<usize> {
        self.possible_spawns
            .iter()
            .enumerate()
            .find_or_last(|(_, elem)| elem.strength >= min_strength)
            .map(|(i, _)| i)
    }

    pub fn compile(&self) -> Vec<CompiledSpawn> {
        let mut spawns = Vec::new();
        for wave in self.waves.iter() {
            spawns.append(&mut wave.compile(self));
        }
        spawns.sort_unstable_by(|a, b| b.spawn_time.cmp(&a.spawn_time));
        spawns
    }
}

pub struct SpawnPoint {
    location: Vec3,
}

#[derive(Clone, Debug)]
pub struct WaveInfo {
    pub strength_mult: f32,
    pub start_time: Duration,
    pub visible: bool,
    pub spawned: bool,
    //sorted in ascending time order
    pub spawns: Vec<WaveSpawn>,
}

impl WaveInfo {
    fn compile(&self, assault: &Assault) -> Vec<CompiledSpawn> {
        let mut spawns = Vec::new();
        for spawn in self.spawns.iter() {
            spawn.compile(assault, &mut spawns, self.start_time, self.strength_mult);
        }
        spawns.sort_unstable_by(|a, b| b.spawn_time.cmp(&a.spawn_time));
        spawns
    }
}

#[derive(Clone, Debug)]
pub struct WaveSpawn {
    pub start_offset: Duration,
    pub spawn: WaveSpawnType,
    pub strategy: SpawnStrategy,
}

impl WaveSpawn {
    fn compile(
        &self,
        assault: &Assault,
        dest: &mut Vec<CompiledSpawn>,
        start_time: Duration,
        strength_mult: f32,
    ) {
        let spawn_time = start_time + self.start_offset;
        self.strategy.compile(spawn_time, |t| match &self.spawn {
            WaveSpawnType::Recursive(spawner) => spawner.compile(assault, dest, t, strength_mult),
            WaveSpawnType::Strength(strength) => {
                let Some(spawn_index) = assault.get_spawn_idx(strength * strength_mult) else {
                    return;
                };
                dest.push(CompiledSpawn {
                    spawn_time: t,
                    spawn_index,
                })
            }
        });
    }
}

#[derive(Clone, Debug)]
pub enum WaveSpawnType {
    Recursive(Box<WaveSpawn>),
    Strength(f32),
}

#[derive(Clone, Copy, Debug)]
pub enum SpawnStrategy {
    Burst { count: u32 },
    Stream { count: u32, delay: Duration },
}

impl SpawnStrategy {
    pub fn compile(self, start_time: Duration, mut spawner: impl FnMut(Duration)) {
        match self {
            SpawnStrategy::Burst { count } => {
                for _ in 0..count {
                    spawner(start_time);
                }
            }
            SpawnStrategy::Stream { count, delay } => {
                for i in 0..count {
                    spawner(start_time + delay * i);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CompiledSpawn {
    spawn_time: Duration,
    spawn_index: usize,
}

pub trait SpawnAction {
    fn spawn(&self, commands: &mut Commands, translation: Vec3);
}

pub struct SpawnableEntity {
    pub strength: f32,
    pub action: Box<dyn SpawnAction + Send + Sync>,
}

fn trigger_assault(
    mut assaults: Query<(Entity, &mut Assault), Without<ActiveAssault>>,
    calendar: Res<Calendar>,
    level: Res<Level>,
    anchor_query: Query<&GlobalTransform, (With<ActiveWorldAnchor>, Without<Assault>)>,
    mut commands: Commands,
) {
    if !calendar.in_night() {
        return; //only trigger these bad boys at night
    }
    for (assault_entity, mut assault) in assaults.iter_mut() {
        assault.spawn_points.clear();
        info!("triggering assault..");
        for tf in anchor_query.iter() {
            info!("creating spawn points...");
            //TODO: should check for a clear area instead of a single block (and be improved in general)
            //      ++ should check downwards so that they don't spawn in the air
            //spawn in circle, check vertical until we find an empty block to spawn on
            const COUNT: i32 = 5;
            const RADIUS: f32 = 25.0;
            const MAX_CHECK: i32 = 100;
            const DELTA_ANGLE: f32 = 2.0 * PI / COUNT as f32;
            const REQUIRED_VOLUME_HALF_EXTENTS: BlockCoord = BlockCoord::new(2, 2, 2);
            let center = tf.translation();
            for i in 0..COUNT {
                let search_origin = BlockCoord::from(
                    center
                        + RADIUS
                            * Vec3::new(
                                (i as f32 * DELTA_ANGLE).cos(),
                                0.0,
                                (i as f32 * DELTA_ANGLE).sin(),
                            ),
                );
                let mut container = VolumeContainer::new(Volume::new(
                    BlockCoord::new(0, 0, 0).into(),
                    BlockCoord::new(0, 0, 0).into(),
                ));
                //try searching up or down to find a potential spawn point
                let potential_spawn_spot = search_for_spawn_volume(
                    &mut container,
                    search_origin,
                    BlockCoord::new(0, 1, 0),
                    true,
                    REQUIRED_VOLUME_HALF_EXTENTS,
                    MAX_CHECK,
                    &level,
                )
                .or(search_for_spawn_volume(
                    &mut container,
                    search_origin,
                    BlockCoord::new(0, 1, 0),
                    true,
                    REQUIRED_VOLUME_HALF_EXTENTS,
                    MAX_CHECK,
                    &level,
                ));
                if let Some(unrefined_spawn_spot) = potential_spawn_spot {
                    //now refine it by searching downward for the lowest possible spawn point
                    let spawn_spot = search_for_spawn_volume(
                        &mut container,
                        BlockCoord::from(unrefined_spawn_spot),
                        BlockCoord::new(0, -1, 0),
                        false,
                        REQUIRED_VOLUME_HALF_EXTENTS,
                        MAX_CHECK,
                        &level,
                    )
                    .unwrap_or(unrefined_spawn_spot);

                    assault.spawn_points.push(SpawnPoint {
                        location: spawn_spot,
                    });
                    info!("created spawn point at {:?}!", spawn_spot);
                } else {
                    //there was no potential spawn point
                    warn!(
                        "Couldn't find a spawn point for search origin: {:?}",
                        search_origin
                    );
                }
            }
            commands.entity(assault_entity).insert(ActiveAssault);
            info!("Assault begins on night {}!", calendar.time.day);
        }
    }
}

fn search_for_spawn_volume(
    container: &mut VolumeContainer<BlockType>,
    search_origin: BlockCoord,
    search_direction: BlockCoord,
    desired_volume_validity: bool,
    volume_half_extents: BlockCoord,
    max_offset: i32,
    level: &Level,
) -> Option<Vec3> {
    let mut offset = BlockCoord::new(0, 0, 0);

    loop {
        let check_volume = Volume::new(
            (search_origin - volume_half_extents + offset).into(),
            (search_origin + volume_half_extents + offset).into(),
        );
        container.recycle(check_volume);
        level.fill_volume_container(container);
        if !desired_volume_validity ^ valid_spawn_volume(container) {
            return Some(check_volume.center());
        }
        if offset.square_magnitude() > max_offset * max_offset {
            return None;
        }
        offset += search_direction;
    }
}

fn valid_spawn_volume(volume: &VolumeContainer<BlockType>) -> bool {
    //all blocks in volume are empty
    volume.iter().all(|(_, b)| {
        b.map(|btype| matches!(btype, BlockType::Empty))
            .unwrap_or(true)
    })
}

fn spawn_wave(
    mut assaults: Query<(Entity, &mut Assault), With<ActiveAssault>>,
    mut wave_event: EventWriter<WaveStartedEvent>,
    mut commands: Commands,
    calendar: Res<Calendar>,
) {
    if !calendar.in_night() {
        //only spawn at night
        return;
    }
    let current_time = calendar.time.time;
    for (assault_entity, mut assault) in assaults.iter_mut() {
        // send wave started events if needed
        for (i, wave) in assault.waves.iter_mut().enumerate() {
            if wave.spawned {
                continue;
            }
            if wave.start_time < current_time {
                //wave just spawned
                wave.spawned = true;
                wave_event.send(WaveStartedEvent(assault_entity, i));
                info!("wave {} spawned", i);
            }
        }
        let spawn_opt = if assault
            .compiled
            .last()
            .is_some_and(|spawn| spawn.spawn_time < current_time)
        {
            assault.compiled.pop()
        } else {
            None
        };
        let Some(spawn_info) = spawn_opt else {
            return;
        };
        let mut rng = thread_rng();
        let Some(spawnpoint) = get_wrapping(&assault.spawn_points, rng.next_u32() as usize) else {
            warn!("no spawnpoint!");
            return;
        };
        if let Some(spawn) = assault.possible_spawns.get(spawn_info.spawn_index) {
            info!("spawning entity with strength {}", spawn.strength);
            spawn.action.spawn(&mut commands, spawnpoint.location);
        }
    }
}

fn despawn_assaults(
    query: Query<Entity, With<ActiveAssault>>,
    anchor_query: Query<(), With<ActiveWorldAnchor>>,
    mut commands: Commands,
    calendar: Res<Calendar>,
) {
    if !calendar.in_night() || anchor_query.is_empty() {
        for assault_entity in query.iter() {
            commands.entity(assault_entity).despawn();
        }
    }
}
