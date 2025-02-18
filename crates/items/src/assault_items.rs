use std::{sync::Arc, time::Duration};

use bevy::prelude::*;

use engine::items::{HitResult, UseEndEvent, UseItemEvent};
use interfaces::scheduling::{GameState, ItemSystemSet};
use waves::waves::{
    spawns::{DefaultSpawn, SkeletonPirateSpawn},
    Assault, SpawnStrategy, SpawnableEntity, WaveInfo, WaveSpawn, WaveSpawnType,
};
use world::atmosphere::Calendar;

// Define a new event for starting an assault
#[derive(Event)]
pub struct StartAssaultEvent;

pub struct AssaultSummonerPlugin;

impl Plugin for AssaultSummonerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StartAssaultEvent>()
            .add_systems(
                Update,
                use_assault_summoner_item.in_set(ItemSystemSet::UsageProcessing),
            )
            .register_type::<AssaultSummonerItem>();
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, FromWorld)]
pub struct AssaultSummonerItem {
    pub strength: f32,
}

fn use_assault_summoner_item(
    mut reader: EventReader<UseItemEvent>,
    mut hit_writer: EventWriter<UseEndEvent>,
    mut commands: Commands,
    query: Query<&AssaultSummonerItem>,
    cal: Res<Calendar>,
) {
    for UseItemEvent {
        user,
        inventory_slot,
        stack,
        tf: _,
    } in reader.read()
    {
        if let Ok(summoner) = query.get(stack.id) {
            // Check if it's night time
            if cal.in_night() {
                info!("Starting assault...");
                let mut assault = Assault::default();
                let start_time = cal.time.time;
                assault.waves.push(WaveInfo {
                    strength_mult: summoner.strength,
                    start_time,
                    visible: true,
                    spawned: false,
                    spawns: vec![
                        WaveSpawn {
                            start_offset: Duration::ZERO,
                            spawn: WaveSpawnType::Strength(10.),
                            strategy: SpawnStrategy::Burst { count: 3 },
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
                assault.waves.push(WaveInfo {
                    strength_mult: summoner.strength,
                    start_time: start_time + Duration::from_secs(60),
                    visible: true,
                    spawned: false,
                    spawns: vec![WaveSpawn {
                        start_offset: Duration::ZERO,
                        spawn: WaveSpawnType::Strength(10.),
                        strategy: SpawnStrategy::Burst { count: 1 },
                    }],
                });
                assault.waves.push(WaveInfo {
                    strength_mult: summoner.strength,
                    start_time: start_time + Duration::from_secs(160),
                    visible: true,
                    spawned: false,
                    spawns: vec![
                        WaveSpawn {
                            start_offset: Duration::ZERO,
                            spawn: WaveSpawnType::Strength(10.),
                            strategy: SpawnStrategy::Burst { count: 1 },
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
                assault.waves.push(WaveInfo {
                    strength_mult: summoner.strength,
                    start_time: start_time + Duration::from_secs(240),
                    visible: true,
                    spawned: false,
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
                let mut spawns = vec![
                    SpawnableEntity {
                        strength: 1.,
                        action: Box::new(SkeletonPirateSpawn),
                    },
                    SpawnableEntity {
                        strength: 10.,
                        action: Box::new(DefaultSpawn(Arc::new("slither_spine".to_string()))),
                    },
                ];
                spawns.sort_by(|a, b| a.strength.total_cmp(&b.strength));
                assault.possible_spawns = spawns;
                assault.compiled = assault.compile();
                commands.spawn((assault, StateScoped(GameState::Game)));
                hit_writer.send(UseEndEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    result: HitResult::Miss, // Successful use
                });
            } else {
                // Item can only be used at night
                info!("Assault Summoner can only be used at night!");
                hit_writer.send(UseEndEvent {
                    user: *user,
                    inventory_slot: *inventory_slot,
                    stack: *stack,
                    result: HitResult::Fail, // Failed use
                });
            }
        }
    }
}
