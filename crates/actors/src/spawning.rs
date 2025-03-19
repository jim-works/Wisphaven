use std::{sync::Arc, time::Duration};

use ahash::HashMap;
use bevy::prelude::*;
use engine::actors::{
    ActorName, ActorRegistry, ActorResources, Combatant, CombatantBundle, Damage, SpawnActorEvent,
    projectile::{Projectile, ProjectileSpawnedInEntity},
    team::*,
};
use interfaces::scheduling::{LevelLoadState, LevelSystemSet};
use physics::{PhysicsBundle, movement::Velocity};
use serde::{Deserialize, Serialize};
use util::SendEventCommand;

pub struct SpawningPlugin;

impl Plugin for SpawningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (named_projectile_spawn_handler, named_actor_spawn_handler)
                .chain()
                .in_set(LevelSystemSet::PostTick),
        )
        .add_event::<SpawnNamedProjectileEvent>()
        .add_event::<SpawnNamedActorEvent>()
        .register_type::<ProjectileName>();
    }
}

#[derive(Event)]
pub struct SpawnNamedActorEvent {
    pub name: Arc<ActorName>,
    pub spawn_args: SpawnActorEvent<()>,
    pub json_args: Option<Arc<str>>,
}

#[derive(Event)]
pub struct SpawnNamedProjectileEvent {
    pub name: Arc<ProjectileName>,
    pub spawn_args: ProjectileSpawnArgs,
    pub json_args: Option<Arc<str>>,
}

fn named_projectile_spawn_handler(
    mut events: EventReader<SpawnNamedProjectileEvent>,
    mut commands: Commands,
    registry: Res<ProjectileRegistry>,
) {
    for SpawnNamedProjectileEvent {
        name,
        spawn_args,
        json_args,
    } in events.read()
    {
        registry.spawn(
            name,
            &mut commands,
            spawn_args.clone(),
            json_args.as_deref(),
        );
    }
}

fn named_actor_spawn_handler(
    mut events: EventReader<SpawnNamedActorEvent>,
    mut commands: Commands,
    resources: Res<ActorResources>,
) {
    for SpawnNamedActorEvent {
        name,
        spawn_args,
        json_args,
    } in events.read()
    {
        resources.registry.spawn(
            name,
            &mut commands,
            spawn_args.transform,
            json_args.as_deref(),
        );
    }
}

#[derive(
    Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component, FromWorld)]
pub struct ProjectileName {
    pub namespace: String,
    pub name: String,
}

impl ProjectileName {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }
    pub fn core(name: impl Into<String>) -> Self {
        Self::new("core", name)
    }
}

//projectile ids may not be stable across program runs. to get a specific id for an actor,
// use actor registry
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectileId(pub usize);

pub type ProjectileNameIdMap = HashMap<ProjectileName, ProjectileId>;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ProjectileSpawnArgs {
    pub transform: Transform,
    pub velocity: Velocity,
    pub combat: CombatantBundle,
    pub owner: Option<Entity>,
    pub damage: Damage,
    pub lifetime: f32,
    pub knockback: f32,
    pub terrain_damage: f32,
}

impl ProjectileSpawnArgs {
    pub fn new(owner: Option<Entity>, team: Team, transform: Transform) -> Self {
        Self {
            owner,
            transform,
            velocity: default(),
            damage: default(),
            combat: CombatantBundle {
                combatant: Combatant::new(10.0, 0.),
                team,
                ..default()
            },
            lifetime: 1.,
            knockback: 1.,
            terrain_damage: 1.,
        }
    }

    pub fn spawn(&self, ec: &mut EntityCommands, curr_time: Duration) -> Projectile {
        let proj = Projectile {
            owner: self.owner,
            knockback_mult: self.knockback,
            terrain_damage: self.terrain_damage,
            despawn_time: curr_time + Duration::from_secs_f32(self.lifetime),
            damage: self.damage,
            ..default()
        };
        ec.insert((
            StateScoped(LevelLoadState::Loaded),
            self.transform,
            self.combat.clone(),
            PhysicsBundle {
                velocity: self.velocity,
                ..default()
            },
            proj.clone(),
        ));
        if let Some(owner) = self.owner {
            ec.insert(ProjectileSpawnedInEntity(owner));
        }
        proj
    }
}

#[derive(Event, Default, Serialize, Deserialize)]
pub struct SpawnProjectileEvent<T> {
    pub args: ProjectileSpawnArgs,
    pub event: T,
}

impl<T> SpawnProjectileEvent<T> {
    pub fn new(owner: Option<Entity>, owner_team: Team, transform: Transform, event: T) -> Self {
        Self {
            args: ProjectileSpawnArgs::new(owner, owner_team, transform),
            event,
        }
    }
}

#[derive(Resource, Default)]
pub struct ProjectileRegistry {
    pub dynamic_generators:
        Vec<Box<dyn Fn(&mut Commands, ProjectileSpawnArgs, Option<&str>) + Send + Sync>>,
    //ids may not be stable across program runs
    pub id_map: ProjectileNameIdMap,
}

impl ProjectileRegistry {
    pub fn add_dynamic<
        T: std::fmt::Debug + Default + for<'de> Deserialize<'de> + Send + Sync + 'static,
    >(
        &mut self,
        name: ProjectileName,
    ) {
        let id = ProjectileId(self.dynamic_generators.len());
        self.dynamic_generators
            .push(Box::new(|commands, args, json_args_opt| {
                if let Some(json_args) = json_args_opt
                    && let Ok(event) = serde_json::from_str::<T>(&json_args)
                {
                    info!("parsed json spawn event {:?}", event);
                    commands.queue(SendEventCommand(SpawnProjectileEvent { args, event }));
                } else {
                    info!("using default spawn event");
                    commands.queue(SendEventCommand(SpawnProjectileEvent {
                        args,
                        event: T::default(),
                    }));
                };
            }));
        self.id_map.insert(name, id);
    }
    pub fn get_id(&self, name: &ProjectileName) -> Option<ProjectileId> {
        self.id_map.get(name).copied()
    }
    pub fn spawn(
        &self,
        projectile: &ProjectileName,
        commands: &mut Commands,
        spawn_args: ProjectileSpawnArgs,
        json_args: Option<&str>,
    ) {
        if let Some(projectile_id) = self.get_id(projectile) {
            if let Some(generator) = self.dynamic_generators.get(projectile_id.0) {
                generator(commands, spawn_args, json_args);
            }
        }
    }
    pub fn spawn_id(
        &self,
        projectile_id: ProjectileId,
        commands: &mut Commands,
        spawn_args: ProjectileSpawnArgs,
        json_args: Option<&str>,
    ) {
        if let Some(generator) = self.dynamic_generators.get(projectile_id.0) {
            generator(commands, spawn_args, json_args);
        }
    }
}

pub trait BuildProjectileRegistry {
    fn add_projectile<
        T: std::fmt::Debug + Default + for<'de> Deserialize<'de> + Send + Sync + 'static,
    >(
        &mut self,
        name: ProjectileName,
    ) -> &mut Self;
}

impl BuildProjectileRegistry for App {
    fn add_projectile<
        T: std::fmt::Debug + Default + for<'de> Deserialize<'de> + Send + Sync + 'static,
    >(
        &mut self,
        name: ProjectileName,
    ) -> &mut App {
        self.add_event::<SpawnProjectileEvent<T>>();
        let mut registry = self
            .world_mut()
            .get_resource_or_insert_with(ProjectileRegistry::default);
        registry.add_dynamic::<T>(name);
        self
    }
}
