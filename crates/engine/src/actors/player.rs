use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;
use bevy_quinnet::client::QuinnetClient;
use player_controller::RotateWithMouse;

use crate::{
    actors::{ghost::FloatBoost, team::PlayerTeam, Invulnerability, MoveSpeed},
    chunk_loading::ChunkLoader,
    controllers::*,
    items::{
        inventory::Inventory,
        item_attributes::{ItemSwingSpeed, ItemUseSpeed},
        *,
    },
    mesher::item_mesher::HeldItemResources,
    net::{
        client::ClientState,
        server::{SyncPosition, SyncVelocity},
        ClientMessage, NetworkType, PlayerList, RemoteClient,
    },
    physics::{movement::*, *},
    world::{settings::Settings, *},
};

use super::{
    abilities::{
        dash::Dash,
        stamina::{RestoreStaminaDuringDay, Stamina},
    },
    death_effects::RestoreStaminaOnKill,
    ghost::{spawn_ghost_hand, Float, GhostResources, Handed},
    Combatant, CombatantBundle, Damage, DeathEvent, DeathInfo,
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
                    respawn_local_player,
                ),
            )
            .add_systems(
                Update,
                send_updated_position_client.run_if(in_state(ClientState::Ready)),
            )
            .add_event::<LocalPlayerSpawnedEvent>()
            .add_event::<SpawnLocalPlayerEvent>();
    }
}

fn spawn_remote_player(
    mut commands: Commands,
    joined_query: Query<(Entity, &RemoteClient), Added<RemoteClient>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    clients: Res<PlayerList>,
    settings: Res<Settings>,
    network_type: Res<State<NetworkType>>,
    ghost_resources: Res<GhostResources>,
    held_item_resouces: Res<HeldItemResources>,
) {
    for (entity, RemoteClient(client_id)) in joined_query.iter() {
        info!(
            "Spawned remote player with username: {}",
            &clients.get(*client_id).unwrap().username
        );
        commands.entity(entity).insert((
            Name::new(clients.get(*client_id).cloned().unwrap().username),
            PbrBundle {
                mesh: meshes.add(Capsule3d::default()),
                material: materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.0),
                    ..default()
                }),
                ..default()
            },
        ));
        if let NetworkType::Server = network_type.get() {
            commands.entity(entity).insert(ChunkLoader {
                mesh: false,
                ..settings.player_loader.clone()
            });
        }
        populate_player_entity(
            entity,
            Vec3::ZERO,
            &ghost_resources,
            &held_item_resouces,
            &mut commands,
        );
    }
}

fn trigger_local_player_spawn(mut writer: EventWriter<SpawnLocalPlayerEvent>) {
    writer.send(SpawnLocalPlayerEvent);
}

fn respawn_local_player(
    mut components: RemovedComponents<LocalPlayer>,
    mut writer: EventWriter<SpawnLocalPlayerEvent>,
) {
    if !components.is_empty() {
        components.clear();
        writer.send(SpawnLocalPlayerEvent);
    }
}

pub fn spawn_local_player(
    mut spawn_reader: EventReader<SpawnLocalPlayerEvent>,
    mut commands: Commands,
    settings: Res<Settings>,
    level: Res<Level>,
    mut pickup_item: EventWriter<PickupItemEvent>,
    mut equip_item: EventWriter<EquipItemEvent>,
    mut spawn_event: EventWriter<LocalPlayerSpawnedEvent>,
    resources: Res<ItemResources>,
    item_query: Query<&MaxStackSize>,
    ghost_resources: Res<GhostResources>,
    held_item_resouces: Res<HeldItemResources>,
    player_query: Query<(), With<LocalPlayer>>,
) {
    for _ in spawn_reader.read() {
        if !player_query.is_empty() {
            info!("trying to spawn local player when there's already one!");
        }
        info!("Spawning local player!");
        //adjust for ghost height
        let spawn_point = level.get_spawn_point() + Vec3::new(0., 1.5, 0.);
        let player_id = commands
            .spawn((
                Name::new("local player"),
                LocalPlayer {},
                CombatantBundle::<PlayerTeam> {
                    combatant: Combatant::new(1.0, 0.0),
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
                settings.player_loader.clone(),
            ))
            .id();
        populate_player_entity(
            player_id,
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
                    .get_basic(&ItemName::core("ruby_hammer"))
                    .unwrap(),
                1,
            ),
            &item_query,
            &mut pickup_item,
            &mut equip_item,
        );
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
            &mut equip_item,
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
            &mut equip_item,
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
            &mut equip_item,
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
            &mut equip_item,
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
            &mut equip_item,
        );
        inventory.pickup_item(
            ItemStack::new(
                resources
                    .registry
                    .get_basic(&ItemName::core("coin_launcher"))
                    .unwrap(),
                100,
            ),
            &item_query,
            &mut pickup_item,
            &mut equip_item,
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
            &mut equip_item,
        );

        commands.entity(player_id).insert(inventory);
        spawn_event.send(LocalPlayerSpawnedEvent(player_id));
    }
}

fn populate_player_entity(
    entity: Entity,
    spawn_point: Vec3,
    ghost_resources: &GhostResources,
    held_item_resources: &HeldItemResources,
    commands: &mut Commands,
) {
    commands.entity(entity).insert((
        Player {
            hit_damage: Damage::new(1.0),
        },
        SpatialBundle::from_transform(Transform::from_translation(spawn_point)),
        //TODO - FIX THIS - not working after upgrade or adding physicslevelset for some reason
        // InterpolatedAttribute::from(Transform::from_translation(spawn_point)),
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
        SyncPosition,
        SyncVelocity,
        Stamina::new(10.0),
        RestoreStaminaOnKill { amount: 1.0 },
        RestoreStaminaDuringDay {
            per_tick: 1. / (64. * 16.),
        },
        Dash::new(0.5, Duration::from_secs_f32(0.5)),
    ));
    //right hand
    let right_hand = spawn_ghost_hand(
        entity,
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
        entity,
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

fn send_updated_position_client(
    client: Res<QuinnetClient>,
    query: Query<(&Transform, &Velocity), With<LocalPlayer>>,
) {
    for (tf, v) in query.iter() {
        client
            .connection()
            .send_message_on(
                0,
                ClientMessage::UpdatePosition {
                    transform: *tf,
                    velocity: v.0,
                },
            )
            .unwrap();
    }
}

fn handle_disconnect(mut commands: Commands, mut removed: RemovedComponents<RemoteClient>) {
    for entity in removed.read() {
        //TODO: make this better
        commands.entity(entity).remove::<(
            SyncPosition,
            SyncVelocity,
            Name,
            PbrBundle,
            PhysicsBundle,
            Player,
        )>();
        info!("Cleaned up disconnected player");
    }
}
