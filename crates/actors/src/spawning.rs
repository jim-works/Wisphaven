use std::sync::Arc;

use ahash::HashMap;
use bevy::prelude::*;
use engine::world::LevelSystemSet;
use util::SendEventCommand;

pub struct SpawningPlugin;

impl Plugin for SpawningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, spawn_handler.in_set(LevelSystemSet::PostTick))
            .add_event::<DefaultSpawnEvent>()
            .add_event::<SpawnActorEvent>();
    }
}

#[derive(Event, Clone)]
pub struct DefaultSpawnEvent {
    pub transform: Transform,
}

#[derive(Event)]
pub struct SpawnActorEvent {
    name: Arc<String>,
    args: DefaultSpawnEvent,
}

#[derive(Resource, Default)]
pub struct ActorRegistry {
    spawners: HashMap<String, Box<dyn ActorSpawner>>,
}

//todo - fix this
//was trying to do something similar to app.add_event::<T>(), but can't figure out how to implement that on a trait
trait ActorSpawner: Fn(&DefaultSpawnEvent, &mut Commands) + Sync + Send {}
impl<T: Fn(&DefaultSpawnEvent, &mut Commands) + Sync + Send> ActorSpawner for T {}

pub trait BuildActorRegistry<Event> {
    fn add_actor(&mut self, name: String) -> &mut Self;
}

impl<Event: for<'a> From<&'a DefaultSpawnEvent> + bevy::prelude::Event> BuildActorRegistry<Event>
    for App
{
    fn add_actor(&mut self, name: String) -> &mut App {
        let mut registry = self
            .world_mut()
            .get_resource_or_insert_with(|| ActorRegistry::default());
        registry.spawners.insert(
            name,
            Box::new(|event: &DefaultSpawnEvent, commands: &mut Commands| {
                commands.add(SendEventCommand(Event::from(event)));
            }),
        );
        self
    }
}

fn spawn_handler(
    mut events: EventReader<SpawnActorEvent>,
    mut commands: Commands,
    registry: Res<ActorRegistry>,
) {
    for SpawnActorEvent { name, args } in events.read() {
        let name: &String = &name;
        if let Some(spawner) = registry.spawners.get(name) {
            spawner(args, &mut commands);
        }
    }
}
