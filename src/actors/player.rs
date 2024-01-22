use std::{f32::consts::PI, time::Duration};

use bevy::{
    core_pipeline::Skybox,
    prelude::*,
    render::{camera::CameraProjection, primitives::Frustum},
};
use bevy_quinnet::client::Client;
use leafwing_input_manager::InputManagerBundle;

use crate::{
    chunk_loading::ChunkLoader,
    controllers::{self, *},
    items::{
        inventory::Inventory,
        item_attributes::{ItemSwingSpeed, ItemUseSpeed},
        *,
    },
    net::{
        client::ClientState,
        server::{SyncPosition, SyncVelocity},
        ClientMessage, NetworkType, PlayerList, RemoteClient,
    },
    physics::{movement::*, *},
    world::{atmosphere::SkyboxCubemap, settings::Settings, *},
};

use super::{CombatInfo, CombatantBundle, Damage, DeathInfo};

#[derive(Component)]
pub struct Player {
    pub hit_damage: Damage,
}

#[derive(Component)]
pub struct LocalPlayer;

#[derive(Event)]
pub struct LocalPlayerSpawnedEvent(pub Entity);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(LevelLoadState::Loaded), spawn_local_player)
            .add_systems(Update, (spawn_remote_player, handle_disconnect))
            .add_systems(
                Update,
                send_updated_position_client.run_if(in_state(ClientState::Ready)),
            )
            .add_event::<LocalPlayerSpawnedEvent>();
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
) {
    for (entity, RemoteClient(client_id)) in joined_query.iter() {
        info!(
            "Spawned remote player with username: {}",
            &clients.get(*client_id).unwrap().username
        );
        commands.entity(entity).insert((
            Name::new(clients.get(*client_id).cloned().unwrap().username),
            PbrBundle {
                mesh: meshes.add(shape::Capsule::default().into()),
                material: materials.add(StandardMaterial {
                    base_color: Color::RED,
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
        populate_player_entity(entity, Vec3::ZERO, &mut commands);
    }
}

pub fn spawn_local_player(
    mut commands: Commands,
    settings: Res<Settings>,
    level: Res<Level>,
    mut pickup_item: EventWriter<PickupItemEvent>,
    mut equip_item: EventWriter<EquipItemEvent>,
    mut spawn_event: EventWriter<LocalPlayerSpawnedEvent>,
    resources: Res<ItemResources>,
    item_query: Query<&MaxStackSize>,
    skybox: Res<SkyboxCubemap>,
) {
    info!("Spawning local player!");
    let mut spawn_point = level.spawn_point;
    const MAX_CHECK_RANGE: i32 = 1000;
    for _ in 0..MAX_CHECK_RANGE {
        match level.get_block(spawn_point.into()) {
            Some(BlockType::Empty) => {
                if let Some(BlockType::Empty) =
                    level.get_block(BlockCoord::from(spawn_point) + BlockCoord::new(0, -1, 0))
                {
                    break;
                }
            }
            Some(_) => {
                spawn_point.y += 1.0;
            }
            None => {
                break;
            } //into unloaded chunks
        }
    }
    let player_id = commands
        .spawn((
            Name::new("local player"),
            LocalPlayer {},
            CombatantBundle {
                combat_info: CombatInfo::new(10.0, 0.0),
                death_info: DeathInfo {
                    death_type: crate::actors::DeathType::LocalPlayer,
                },
            },
            RotateWithMouse {
                lock_pitch: true,
                ..default()
            },
            ControllableBundle { ..default() },
            settings.player_loader.clone(),
            InputManagerBundle {
                input_map: controllers::get_input_map(),
                ..default()
            },
        ))
        .id();
    populate_player_entity(player_id, spawn_point, &mut commands);
    let mut inventory = Inventory::new(player_id, 40);

    inventory.pickup_item(
        ItemStack::new(
            resources
                .registry
                .get_basic(&ItemName::core("world_anchor"))
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
                .get_basic(&ItemName::core("glowjelly_jar"))
                .unwrap(),
            100,
        ),
        &item_query,
        &mut pickup_item,
        &mut equip_item,
    );

    commands.entity(player_id).insert(inventory);
    let projection = PerspectiveProjection {
        fov: PI / 2.,
        far: 1_000_000_000.0,
        ..default()
    };
    spawn_event.send(LocalPlayerSpawnedEvent(player_id));
    commands.spawn((
        Name::new("Camera"),
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 1.5, 0.0),
            projection: Projection::Perspective(projection.clone()),
            frustum: Frustum::from_view_projection(&projection.get_projection_matrix()),
            ..default()
        },
        FogSettings {
            color: Color::rgba(0.56, 0.824, 1.0, 1.0),
            // directional_light_color: Color::rgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 0.8,
            falloff: FogFalloff::Linear {
                start: 100.0,
                end: 200.0,
            },
            ..default()
        },
        Skybox(skybox.0.clone()),
        RotateWithMouse {
            pitch_bound: PI * 0.49,
            lock_yaw: true,
            ..default()
        },
        FollowPlayer {
            offset: Vec3::new(0.0, 1.5, 0.0),
        },
        PlayerActionOrigin {},
        InputManagerBundle {
            input_map: controllers::get_input_map(),
            ..default()
        },
    ));
}

fn populate_player_entity(entity: Entity, spawn_point: Vec3, commands: &mut Commands) {
    commands.entity(entity).insert((
        Player {
            hit_damage: Damage { amount: 1.0 },
        },
        TransformBundle::from_transform(Transform::from_translation(spawn_point)),
        InterpolatedAttribute::from(Transform::from_translation(spawn_point)),
        PhysicsBundle {
            collider: collision::Aabb::new(Vec3::new(0.8, 1.6, 0.8), Vec3::new(-0.4, 0.0, -0.4)),
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
    ));
}

fn send_updated_position_client(
    client: Res<Client>,
    query: Query<(&Transform, &Velocity), With<LocalPlayer>>,
) {
    for (tf, v) in query.iter() {
        client
            .connection()
            .send_message_on(
                bevy_quinnet::shared::channel::ChannelId::Unreliable,
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
