use std::sync::Arc;

use ahash::HashMap;
use bevy::prelude::*;
use engine::{
    actors::{team::*, Combatant, CombatantBundle, Damage},
    all_teams_function, all_teams_system,
    physics::movement::Velocity,
    world::LevelSystemSet,
};
use util::SendEventCommand;

pub struct SpawningPlugin;

impl Plugin for SpawningPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnActorEvent>().add_systems(
            FixedUpdate,
            (
                actor_spawn_handler,
                all_teams_system!(projectile_spawn_handler),
            )
                .in_set(LevelSystemSet::PostTick),
        );
        all_teams_function!(app, add_event, SpawnProjectileEvent);
    }
}

#[derive(Clone, Copy, Default)]
pub struct DefaultSpawnArgs {
    pub transform: Transform,
}

#[derive(Clone)]
pub struct ProjectileSpawnArgs<T: Team> {
    pub velocity: Velocity,
    pub combat: CombatantBundle<T>,
    pub owner: Entity,
    pub damage: Damage,
    pub lifetime_mult: f32,
    pub knockback_mult: f32,
    pub terrain_damage_mult: f32,
}

impl<T: Team> ProjectileSpawnArgs<T> {
    pub fn new(owner: Entity) -> Self {
        Self {
            owner,
            velocity: default(),
            damage: default(),
            combat: CombatantBundle::<T> {
                combatant: Combatant::new(10.0, 0.),
                ..default()
            },
            lifetime_mult: 1.,
            knockback_mult: 1.,
            terrain_damage_mult: 1.,
        }
    }
}

#[derive(Event)]
pub struct SpawnActorEvent {
    pub name: Arc<String>,
    pub args: DefaultSpawnArgs,
}

#[derive(Event)]
pub struct SpawnProjectileEvent<T: Team> {
    pub name: Arc<String>,
    pub default: DefaultSpawnArgs,
    pub projectile: ProjectileSpawnArgs<T>,
}

#[derive(Resource, Default)]
pub struct ActorRegistry {
    spawners: HashMap<String, Box<dyn ActorSpawner>>,
}

trait ActorSpawner: Fn(DefaultSpawnArgs, &mut Commands) + Sync + Send {}
impl<T: Fn(DefaultSpawnArgs, &mut Commands) + Sync + Send> ActorSpawner for T {}

#[derive(Resource, Default)]
pub struct ProjectileRegistry<T: Team> {
    spawners: HashMap<String, Box<dyn ProjectileSpawner<T>>>,
}

trait ProjectileSpawner<T>:
    Fn(DefaultSpawnArgs, ProjectileSpawnArgs<T>, &mut Commands) + Sync + Send
{
}
impl<T: Team, S: Fn(DefaultSpawnArgs, ProjectileSpawnArgs<T>, &mut Commands) + Sync + Send>
    ProjectileSpawner<T> for S
{
}

pub trait BuildActorRegistry {
    fn add_actor<Event: From<DefaultSpawnArgs> + bevy::prelude::Event>(
        &mut self,
        name: String,
    ) -> &mut Self;
}

impl BuildActorRegistry for App {
    fn add_actor<Event: From<DefaultSpawnArgs> + bevy::prelude::Event>(
        &mut self,
        name: String,
    ) -> &mut App {
        let mut registry = self
            .world_mut()
            .get_resource_or_insert_with(ActorRegistry::default);
        registry.spawners.insert(
            name,
            Box::new(|event: DefaultSpawnArgs, commands: &mut Commands| {
                commands.queue(SendEventCommand(Event::from(event)));
            }),
        );
        self
    }
}

fn actor_spawn_handler(
    mut events: EventReader<SpawnActorEvent>,
    mut commands: Commands,
    registry: Res<ActorRegistry>,
) {
    for SpawnActorEvent { name, args } in events.read() {
        let name: &String = name;
        if let Some(spawner) = registry.spawners.get(name) {
            spawner(*args, &mut commands);
        }
    }
}

pub trait BuildProjectileRegistry<T: Team> {
    fn add_projectile<
        Event: From<(DefaultSpawnArgs, ProjectileSpawnArgs<T>)> + bevy::prelude::Event,
    >(
        &mut self,
        name: String,
    ) -> &mut Self;
}

impl<T: Team> BuildProjectileRegistry<T> for App {
    fn add_projectile<
        Event: From<(DefaultSpawnArgs, ProjectileSpawnArgs<T>)> + bevy::prelude::Event,
    >(
        &mut self,
        name: String,
    ) -> &mut App {
        let mut registry = self
            .world_mut()
            .get_resource_or_insert_with(ProjectileRegistry::default);
        registry.spawners.insert(
            name,
            Box::new(
                |default: DefaultSpawnArgs,
                 event: ProjectileSpawnArgs<T>,
                 commands: &mut Commands| {
                    commands.queue(SendEventCommand(Event::from((default, event))));
                },
            ),
        );
        self
    }
}

fn projectile_spawn_handler<T: Team>(
    mut events: EventReader<SpawnProjectileEvent<T>>,
    mut commands: Commands,
    registry: Res<ProjectileRegistry<T>>,
) {
    for SpawnProjectileEvent {
        name,
        default: default_args,
        projectile: projectile_args,
    } in events.read()
    {
        let name: &String = name;
        if let Some(spawner) = registry.spawners.get(name) {
            spawner(*default_args, projectile_args.clone(), &mut commands);
        }
    }
}
