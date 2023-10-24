use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;
use rand::{thread_rng, RngCore};

use crate::{
    actors::world_anchor::WorldAnchor,
    world::{
        atmosphere::{Calendar, NightStartedEvent},
        BlockCoord, Level,
    }, util::get_wrapping,
};

use self::spawns::SkeletonPirateSpawn;

pub mod spawns;

pub struct WavesPlugin;

impl Plugin for WavesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                trigger_assault.run_if(resource_exists::<WorldAnchor>()),
                spawn_wave.run_if(
                    in_state(AssaultState::Spawning).and_then(resource_exists::<Assault>()),
                ),
            ),
        )
        .insert_resource(Assault {
            to_spawn: Vec::new(),
            possible_spawns: vec![Spawn { strength: 1, wave_strength_cutoff: None, action: Box::new(SkeletonPirateSpawn) }],
            spawn_points: Vec::new(),
            wave_pause_interval: Duration::from_secs(5)
        })
        .add_state::<AssaultState>();
    }
}

#[derive(States, Default, Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub enum AssaultState {
    #[default]
    NotStarted,
    Spawning,
    Finished,
}

#[derive(Resource)]
pub struct Assault {
    pub to_spawn: Vec<WaveInfo>,
    pub possible_spawns: Vec<Spawn>,
    pub spawn_points: Vec<SpawnPoint>,
    pub wave_pause_interval: Duration,
}

pub struct SpawnPoint {
    location: Vec3,
}

#[derive(Clone, Debug)]
pub struct WaveInfo {
    pub initial_strength: u64,
    pub remaining_strength: u64,
    pub duration: Duration,
    pub end_time: Duration,
    pub next_spawn_time: Duration,
}

impl WaveInfo {
    fn new(strength: u64, duration: Duration) -> Self {
        Self {
            initial_strength: strength,
            remaining_strength: strength,
            duration,
            end_time: Duration::ZERO,
            next_spawn_time: Duration::ZERO,
        }
    }
}

pub trait SpawnAction {
    fn spawn(&self, commands: &mut Commands, translation: Vec3);
}

pub struct Spawn {
    pub strength: u64,
    pub wave_strength_cutoff: Option<u64>, //don't use this spawn if wave strength is larger
    pub action: Box<dyn SpawnAction + Send + Sync>,
}

impl Spawn {
    pub fn usable(&self, wave_strength: u64) -> bool {
        self.wave_strength_cutoff
            .map(|cutoff| cutoff <= wave_strength)
            .unwrap_or(true)
    }
}

fn trigger_assault(
    mut assault: ResMut<Assault>,
    mut next_state: ResMut<NextState<AssaultState>>,
    mut night_event: EventReader<NightStartedEvent>,
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
        let center = tf.translation();
        for i in 0..COUNT {
            let spawn_point = BlockCoord::from(
                center
                    + RADIUS
                        * Vec3::new(
                            (i as f32 * DELTA_ANGLE).cos(),
                            0.0,
                            (i as f32 * DELTA_ANGLE).sin(),
                        ),
            );
            let mut check_point = spawn_point;
            while check_point.y - spawn_point.y < MAX_CHECK
                && level.get_block_entity(check_point).is_some()
            {
                check_point.y += 1;
            }
            if level.get_block_entity(check_point).is_none() {
                //we have a clear spot to spawn
                assault.spawn_points.push(SpawnPoint {
                    location: spawn_point.center(),
                });
                info!("added spawnpoint at {:?}!", spawn_point.center());
            }
        }
        next_state.set(AssaultState::Finished);
    }
    info!("Assault begins on night {}!", calendar.time.day);
    assault.to_spawn.push(WaveInfo::new(
        get_wave_strength(&calendar),
        Duration::from_secs(2),
    ));
    assault.to_spawn.push(WaveInfo::new(
        get_wave_strength(&calendar),
        Duration::from_secs(2),
    ));
    next_state.set(AssaultState::Spawning);
}

fn get_wave_strength(calendar: &Calendar) -> u64 {
    calendar.time.day + 5
}

fn spawn_wave(
    mut assault: ResMut<Assault>,
    time: Res<Time>,
    mut commands: Commands,
    mut assault_state: ResMut<NextState<AssaultState>>,
) {
    let current_time = time.elapsed();
    let mut rng = thread_rng();
    if let Some(mut wave) = assault.to_spawn.last().cloned() {
        if wave.next_spawn_time != Duration::ZERO && wave.next_spawn_time > current_time {
            return;
        }
        if wave.end_time == Duration::ZERO && wave.remaining_strength == 0 {
            wave.end_time = current_time;
            let idx = assault.to_spawn.len() - 1;
            assault.to_spawn[idx] = wave;
            info!("finished wave!");
            return;
        }
        if wave.end_time != Duration::ZERO && current_time > wave.end_time + assault.wave_pause_interval {
            assault.to_spawn.pop();
            info!("done waiting after wave pause");
            return;
        }
        let possible_spawns: Vec<&Spawn> = assault
            .possible_spawns
            .iter()
            .filter(|spawn| {
                spawn.usable(wave.initial_strength) && spawn.strength <= wave.remaining_strength
            })
            .collect();
        if let Some(spawn) = get_wrapping(&possible_spawns, rng.next_u32() as usize) {
            if let Some(spawnpoint) =  get_wrapping(&assault.spawn_points, rng.next_u32() as usize) {
                wave.remaining_strength -= spawn.strength;
                spawn.action.spawn(&mut commands, spawnpoint.location);
                wave.next_spawn_time = current_time + wave.duration.mul_f32(spawn.strength as f32 / wave.initial_strength as f32);
                let idx = assault.to_spawn.len() - 1;
                info!("spawn strength {:?} wave {:?}", spawn.strength, wave);
                assault.to_spawn[idx] = wave;
            } else {
                warn!("no spawnpoint for wave");
            }
        } else {
            //no possible spawns, make sure wave is finished
            wave.remaining_strength = 0;
            return;
        }
    } else {
        assault_state.set(AssaultState::Finished);
        info!("assault finished");
    }
}