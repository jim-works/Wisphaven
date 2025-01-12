use std::{f32::consts::PI, time::Duration};

use ::util::SendEventCommand;
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::client::Predicted;
use player_controller::RotateWithMouse;

use crate::{
    actors::{ghost::FloatBoost, team::PlayerTeam, Invulnerability, MoveSpeed},
    camera::MainCamera,
    chunk_loading::ChunkLoader,
    controllers::*,
    items::{
        inventory::Inventory,
        item_attributes::{ItemSwingSpeed, ItemUseSpeed},
        *,
    },
    mesher::item_mesher::HeldItemResources,
    net::{NetworkType, PlayerList, RemoteClient},
    physics::*,
    world::{settings::Settings, *},
};

use super::{
    abilities::{
        dash::Dash,
        stamina::{RestoreStaminaDuringDay, Stamina},
    },
    death_effects::RestoreStaminaOnKill,
    ghost::{spawn_ghost_hand, Float, GhostResources, Handed},
    world_anchor::ActiveWorldAnchor,
    Combatant, CombatantBundle, Damage, DeathInfo,
};

#[derive(Component)]
pub struct Player {
    pub hit_damage: Damage,
}

#[derive(Component)]
pub struct LocalPlayer;

#[derive(Event)]
pub struct LocalPlayerSpawnedEvent(pub Entity);

#[derive(Event)]
pub struct SpawnLocalPlayerEvent;

#[derive(Resource)]
pub struct RespawningPlayer(pub Option<Duration>);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(LevelLoadState::Loaded), trigger_local_player_spawn)
            .add_systems(
                Update,
                (
                    (spawn_local_player, spawn_remote_player)
                        .run_if(resource_exists::<HeldItemResources>),
                    handle_disconnect,
                ),
            )
            .add_systems(
                FixedUpdate,
                (
                    queue_players_for_respawn.run_if(resource_exists::<ActiveWorldAnchor>),
                    respawn_players,
                )
                    .chain()
                    .in_set(LevelSystemSet::PreTick),
            )
            .add_event::<LocalPlayerSpawnedEvent>()
            .add_event::<SpawnLocalPlayerEvent>()
            .insert_resource(RespawningPlayer(None));
    }
}

fn spawn_remote_player(
    mut commands: Commands,
    joined_query: Query<(Entity, &RemoteClient), (Without<Player>, Without<Predicted>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<Settings>,
    network_type: Res<State<NetworkType>>,
    ghost_resources: Res<GhostResources>,
    held_item_resouces: Res<HeldItemResources>,
    camera: Res<MainCamera>,
    level: Res<Level>,
) {
    for (entity, RemoteClient(client_id)) in joined_query.iter() {
        info!("spawning remote player {:?}", client_id);
        // info!(
        //     "Spawned remote player with username: {}",
        //     &clients.get(client_id).unwrap().username
        // );
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Capsule3d::default())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                ..default()
            })),
        ));
        if network_type.get().is_server() {
            commands.entity(entity).insert((
                ChunkLoader {
                    mesh: false,
                    ..settings.player_loader.clone()
                },
                Name::new(format!("player_{:?}", client_id)),
            ));
        }
        populate_player_entity(
            entity,
            camera.0,
            level.get_spawn_point(),
            &ghost_resources,
            &held_item_resouces,
            &mut commands,
        );
    }
}

fn trigger_local_player_spawn(mut writer: EventWriter<SpawnLocalPlayerEvent>) {
    writer.send(SpawnLocalPlayerEvent);
}

//todo - update when I update mulitplayer
fn queue_players_for_respawn(
    mut components: RemovedComponents<LocalPlayer>,
    mut respawning: ResMut<RespawningPlayer>,
    time: Res<Time>,
) {
    if !components.is_empty() {
        components.clear();
        respawning.0 = Some(time.elapsed() + Duration::from_secs(5));
    }
}

//todo - update when I update mulitplayer
// this needs to always run, the game over transition doesn't happen if there's a player pending respawn
fn respawn_players(
    mut writer: EventWriter<SpawnLocalPlayerEvent>,
    mut respawning: ResMut<RespawningPlayer>,
    time: Res<Time>,
) {
    if respawning
        .0
        .map(|respawn_time| time.elapsed() >= respawn_time)
        .unwrap_or(false)
    {
        info!("Respawning player!");
        writer.send(SpawnLocalPlayerEvent);
        respawning.0 = None;
    }
}

pub(crate) fn spawn_local_player(
    mut spawn_reader: EventReader<SpawnLocalPlayerEvent>,
    network_type: Res<State<NetworkType>>,
    mut commands: Commands,
    settings: Res<Settings>,
    level: Res<Level>,
    mut pickup_item: EventWriter<PickupItemEvent>,
    resources: Res<ItemResources>,
    item_query: Query<&MaxStackSize>,
    ghost_resources: Res<GhostResources>,
    held_item_resouces: Res<HeldItemResources>,
    player_query: Query<(), With<LocalPlayer>>,
    client_player_query: Query<Entity, (With<Predicted>, Without<Player>)>,
    camera: Res<MainCamera>,
) {
    if !matches!(network_type.get(), NetworkType::Client) && spawn_reader.is_empty() {
        return;
    }
    if matches!(network_type.get(), NetworkType::Client) && client_player_query.is_empty() {
        // player already spawned
        return;
    }
    if matches!(network_type.get(), NetworkType::Client) && client_player_query.iter().len() != 1 {
        warn!(
            "invalid clinet player count: {}",
            client_player_query.iter().len()
        );
        return;
    }
    if !player_query.is_empty() {
        info!("trying to spawn local player when there's already one!");
    }
    spawn_reader.clear();
    //adjust for ghost height
    let spawn_point = level.get_spawn_point() + Vec3::new(0., 1.5, 0.);
    info!("Spawning local player at {:?}", spawn_point);
    // decide if we need to spawn an entity or can use the one provided by the server
    let player_id = if let Ok(e) = client_player_query.get_single() {
        info!("using remote");
        e
    } else {
        info!("using local");
        commands.spawn_empty().id()
    };
    info!("network type: {:?}", network_type.get());
    commands.entity(player_id).insert((
        StateScoped(LevelLoadState::Loaded),
        Name::new("local player"),
        LocalPlayer {},
        CombatantBundle::<PlayerTeam> {
            combatant: Combatant::new(10.0, 0.0),
            death_info: DeathInfo {
                death_type: crate::actors::DeathType::LocalPlayer,
            },
            invulnerability: Invulnerability::new(Duration::from_secs(1)),
            ..default()
        },
        RotateWithMouse {
            pitch_bound: PI * 0.49,
            ..default()
        },
        ControllableBundle {
            move_speed: MoveSpeed::new(0.5, 0.5, 0.10),
            ..default()
        },
        FloatBoost::default().with_extra_height(3.0),
        ActionState::<Action>::default(),
        settings.player_loader.clone(),
    ));
    populate_player_entity(
        player_id,
        camera.0,
        spawn_point,
        &ghost_resources,
        &held_item_resouces,
        &mut commands,
    );
    let mut inventory = Inventory::new(player_id, 40);

    inventory.pickup_item(
        ItemStack::new(
            resources
                .registry
                .get_basic(&ItemName::core("ruby_pickaxe"))
                .unwrap(),
            1,
        ),
        &item_query,
        &mut pickup_item,
    );
    inventory.pickup_item(
        ItemStack::new(
            resources
                .registry
                .get_basic(&ItemName::core("ruby_shovel"))
                .unwrap(),
            1,
        ),
        &item_query,
        &mut pickup_item,
    );
    inventory.pickup_item(
        ItemStack::new(
            resources
                .registry
                .get_basic(&ItemName::core("ruby_axe"))
                .unwrap(),
            1,
        ),
        &item_query,
        &mut pickup_item,
    );
    inventory.pickup_item(
        ItemStack::new(
            resources
                .registry
                .get_basic(&ItemName::core("moon"))
                .unwrap(),
            1,
        ),
        &item_query,
        &mut pickup_item,
    );
    inventory.pickup_item(
        ItemStack::new(
            resources
                .registry
                .get_basic(&ItemName::core("dagger"))
                .unwrap(),
            100,
        ),
        &item_query,
        &mut pickup_item,
    );
    inventory.pickup_item(
        ItemStack::new(
            resources
                .registry
                .get_basic(&ItemName::core("spike_ball_launcher"))
                .unwrap(),
            100,
        ),
        &item_query,
        &mut pickup_item,
    );
    inventory.pickup_item(
        ItemStack::new(
            resources
                .registry
                .get_basic(&ItemName::core("grapple"))
                .unwrap(),
            1,
        ),
        &item_query,
        &mut pickup_item,
    );
    inventory.pickup_item(
        ItemStack::new(
            resources
                .registry
                .get_basic(&ItemName::core("suicide_pill"))
                .unwrap(),
            1,
        ),
        &item_query,
        &mut pickup_item,
    );

    commands.entity(player_id).insert(inventory);
    //makes sure that player is actually spawned before this occurs, since events fire at a different time than commands
    commands.queue(SendEventCommand(LocalPlayerSpawnedEvent(player_id)));
}

fn populate_player_entity(
    entity: Entity,
    camera: Entity,
    spawn_point: Vec3,
    ghost_resources: &GhostResources,
    held_item_resources: &HeldItemResources,
    commands: &mut Commands,
) {
    commands.entity(entity).insert((
        Player {
            hit_damage: Damage::new(1.0),
        },
        Transform::from_translation(spawn_point),
        Visibility::default(),
        Float::default(),
        PhysicsBundle {
            collider: collision::Aabb::centered(Vec3::new(0.8, 1.0, 0.8))
                .add_offset(Vec3::new(0.0, -0.3, 0.0)),
            ..default()
        },
        ItemUseSpeed {
            windup: Duration::ZERO,
            backswing: Duration::from_millis(100),
        },
        ItemSwingSpeed {
            windup: Duration::ZERO,
            backswing: Duration::from_millis(100),
        },
        Stamina::new(10.0),
        RestoreStaminaOnKill { amount: 1.0 },
        RestoreStaminaDuringDay {
            per_tick: 1. / (64. * 16.),
        },
        Dash::new(0.5, Duration::from_secs_f32(0.5)),
    ));
    //right hand
    let right_hand = spawn_ghost_hand(
        camera,
        Transform::from_translation(spawn_point),
        Vec3::new(0.7, -0.5, -0.6),
        Vec3::new(0.8, 0.2, -0.5),
        0.15,
        Quat::default(),
        ghost_resources,
        commands,
    );
    //left hand
    let _left_hand = spawn_ghost_hand(
        camera,
        Transform::from_translation(spawn_point),
        Vec3::new(-0.7, -0.5, -0.6),
        Vec3::new(-0.8, 0.2, -0.5),
        0.15,
        Quat::default(),
        ghost_resources,
        commands,
    );
    Handed::Right.assign_hands(entity, right_hand, right_hand, commands);
    let item_visualizer = crate::mesher::item_mesher::create_held_item_visualizer(
        commands,
        entity,
        Transform::from_scale(Vec3::splat(4.0)).with_translation(Vec3::new(0.0, -1.0, -3.4)),
        held_item_resources,
    );
    commands.entity(right_hand).add_child(item_visualizer);
}

fn handle_disconnect(mut commands: Commands, mut removed: RemovedComponents<RemoteClient>) {
    for entity in removed.read() {
        //TODO: make this better
        commands.entity(entity).remove::<(
            Name,
            Mesh3d,
            MeshMaterial3d<StandardMaterial>,
            Transform,
            GlobalTransform,
            PhysicsBundle,
            Player,
        )>();
        info!("Cleaned up disconnected player");
    }
}
