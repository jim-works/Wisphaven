use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;
use itertools::Itertools;
use rand::{thread_rng, RngCore};

use crate::{
    actors::world_anchor::WorldAnchor,
    util::{
        get_wrapping,
        iterators::{BlockVolume, VolumeContainer},
    },
    world::{
        atmosphere::{Calendar, NightStartedEvent},
        BlockCoord, BlockType, Level,
    },
};

use self::spawns::SkeletonPirateSpawn;

pub mod spawns;

pub struct WavesPlugin;

impl Plugin for WavesPlugin {
    fn build(&self, app: &mut App) {
        let mut spawns = vec![SpawnableEntity {
            strength: 1.,
            action: Box::new(SkeletonPirateSpawn),
        }];
        spawns.sort_by(|a, b| a.strength.total_cmp(&b.strength));
        app.add_systems(
            Update,
            (
                trigger_assault.run_if(resource_exists::<WorldAnchor>()),
                spawn_wave.run_if(resource_exists::<Assault>()),
                send_wave_started_event
                    .after(trigger_assault)
                    .run_if(resource_exists::<Assault>()),
            ),
        )
        .insert_resource(Assault {
            to_spawn: Vec::new(),
            compiled: Vec::new(),
            possible_spawns: spawns,
            spawn_points: Vec::new(),
        })
        .add_event::<AssaultStartedEvent>()
        .add_event::<WaveStartedEvent>();
    }
}

#[derive(Event)]
pub struct AssaultStartedEvent;

#[derive(Event)]
pub struct WaveStartedEvent(usize);

#[derive(Resource)]
pub struct Assault {
    pub to_spawn: Vec<WaveInfo>,
    //create by calling .compile(), sorted in descending time order (so you can pop)
    compiled: Vec<CompiledSpawn>,
    pub possible_spawns: Vec<SpawnableEntity>,
    pub spawn_points: Vec<SpawnPoint>,
}

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

    fn compile(&self) -> Vec<CompiledSpawn> {
        let mut spawns = Vec::new();
        for wave in self.to_spawn.iter() {
            spawns.append(&mut wave.compile(&self));
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
    //sorted in ascending time order
    spawns: Vec<WaveSpawn>,
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
struct WaveSpawn {
    start_offset: Duration,
    spawn: WaveSpawnType,
    strategy: SpawnStrategy,
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
enum WaveSpawnType {
    Recursive(Box<WaveSpawn>),
    Strength(f32),
}

#[derive(Clone, Copy, Debug)]
enum SpawnStrategy {
    Burst { count: u32 },
    Stream { count: u32, delay: Duration },
}

impl SpawnStrategy {
    fn compile(self, start_time: Duration, mut spawner: impl FnMut(Duration)) {
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
struct CompiledSpawn {
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
    mut assault: ResMut<Assault>,
    mut night_event: EventReader<NightStartedEvent>,
    mut assault_event: EventWriter<AssaultStartedEvent>,
    calendar: Res<Calendar>,
    level: Res<Level>,
    anchor_query: Query<&GlobalTransform, With<WorldAnchor>>,
) {
    if night_event.is_empty() {
        return;
    }
    night_event.clear();
    assault.spawn_points.clear();
    if let Ok(tf) = anchor_query.get_single() {
        info!("triggering assault...");
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
            let mut container = VolumeContainer::new(BlockVolume::new(
                BlockCoord::new(0, 0, 0),
                BlockCoord::new(0, 0, 0),
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
    }
    info!("Assault begins on night {}!", calendar.time.day);
    assault.to_spawn.clear();
    let strength_mult = get_wave_strength(&calendar);
    let start_time = calendar.time.time + Duration::from_secs_f32(15.);
    assault.to_spawn.push(WaveInfo {
        strength_mult,
        start_time,
        visible: true,
        spawns: vec![
            WaveSpawn {
                start_offset: Duration::ZERO,
                spawn: WaveSpawnType::Strength(1.),
                strategy: SpawnStrategy::Burst { count: 5 },
            },
            WaveSpawn {
                start_offset: Duration::ZERO,
                spawn: WaveSpawnType::Recursive(Box::new(WaveSpawn {
                    start_offset: Duration::from_secs(1),
                    spawn: WaveSpawnType::Strength(1.),
                    strategy: SpawnStrategy::Burst { count: 2 },
                })),
                strategy: SpawnStrategy::Stream {
                    count: 10,
                    delay: Duration::from_secs(5),
                },
            },
        ],
    });
    assault.to_spawn.push(WaveInfo {
        strength_mult,
        start_time: start_time + Duration::from_secs(120),
        visible: true,
        spawns: vec![
            WaveSpawn {
                start_offset: Duration::ZERO,
                spawn: WaveSpawnType::Strength(1.),
                strategy: SpawnStrategy::Burst { count: 5 },
            },
            WaveSpawn {
                start_offset: Duration::ZERO,
                spawn: WaveSpawnType::Recursive(Box::new(WaveSpawn {
                    start_offset: Duration::from_secs(1),
                    spawn: WaveSpawnType::Strength(1.),
                    strategy: SpawnStrategy::Burst { count: 3 },
                })),
                strategy: SpawnStrategy::Stream {
                    count: 3,
                    delay: Duration::from_secs(10),
                },
            },
        ],
    });
    assault.to_spawn.push(WaveInfo {
        strength_mult,
        start_time: start_time + Duration::from_secs(220),
        visible: true,
        spawns: vec![
            WaveSpawn {
                start_offset: Duration::ZERO,
                spawn: WaveSpawnType::Strength(1.),
                strategy: SpawnStrategy::Burst { count: 10 },
            },
            WaveSpawn {
                start_offset: Duration::ZERO,
                spawn: WaveSpawnType::Recursive(Box::new(WaveSpawn {
                    start_offset: Duration::from_secs(1),
                    spawn: WaveSpawnType::Strength(1.),
                    strategy: SpawnStrategy::Burst { count: 5 },
                })),
                strategy: SpawnStrategy::Stream {
                    count: 2,
                    delay: Duration::from_secs(1),
                },
            },
        ],
    });
    assault.to_spawn.push(WaveInfo {
        strength_mult,
        start_time: start_time + Duration::from_secs(400),
        visible: true,
        spawns: vec![
            WaveSpawn {
                start_offset: Duration::ZERO,
                spawn: WaveSpawnType::Strength(1.),
                strategy: SpawnStrategy::Burst { count: 0 },
            },
            WaveSpawn {
                start_offset: Duration::ZERO,
                spawn: WaveSpawnType::Recursive(Box::new(WaveSpawn {
                    start_offset: Duration::from_secs(1),
                    spawn: WaveSpawnType::Strength(1.),
                    strategy: SpawnStrategy::Burst { count: 5 },
                })),
                strategy: SpawnStrategy::Stream {
                    count: 0,
                    delay: Duration::from_secs(10),
                },
            },
        ],
    });
    assault.compiled = assault.compile();
    assault_event.send(AssaultStartedEvent);
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
        let check_volume = BlockVolume::new(
            search_origin - volume_half_extents + offset,
            search_origin + volume_half_extents + offset,
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

fn get_wave_strength(calendar: &Calendar) -> f32 {
    calendar.time.day as f32 + 5.
}

fn spawn_wave(mut assault: ResMut<Assault>, mut commands: Commands, calendar: Res<Calendar>) {
    if !calendar.in_night() {
        //only spawn at night
        return;
    }
    let current_time = calendar.time.time;
    let spawn_opt = if assault
        .compiled
        .last()
        .map_or(false, |spawn| spawn.spawn_time < current_time)
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
        spawn.action.spawn(&mut commands, spawnpoint.location);
    }
}

fn send_wave_started_event(
    assault: Res<Assault>,
    calendar: Res<Calendar>,
    mut assault_event: EventReader<AssaultStartedEvent>,
    mut wave_event: EventWriter<WaveStartedEvent>,
    mut waves_spawned: Local<Vec<bool>>,
    mut all_spawned: Local<bool>,
) {
    if !assault_event.is_empty() {
        assault_event.clear();
        waves_spawned.clear();
        waves_spawned.resize(assault.to_spawn.len(), false);
        *all_spawned = false;
    }
    if *all_spawned {
        return;
    }
    let current_time = calendar.time.time;
    for (i, wave) in assault.to_spawn.iter().enumerate() {
        if !waves_spawned.get(i).unwrap_or(&true) && wave.start_time < current_time {
            //wave just spawned
            waves_spawned[i] = true;
            wave_event.send(WaveStartedEvent(i));
            info!("wave {} spawned", i);
            if waves_spawned.iter().all(|spawned| *spawned) {
                *all_spawned = true;
                info!("all waves spawned!");
            }
        }
    }
}
