//based on https://github.com/Henauxg/bevy_quinnet/blob/main/examples/chat/server.rs

use std::net::IpAddr;

use bevy::{prelude::*, utils::HashSet};
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, ConnectionLostEvent, Endpoint, QuinnetServerPlugin,
        Server, ServerConfiguration,
    },
    shared::{channel::ChannelId, ClientId},
};

use crate::{
    actors::LocalPlayer,
    items::{ItemRegistry, ItemResources, UseItemEvent, SwingItemEvent, inventory::Inventory},
    net::{DisconnectedClient, PlayerInfo, PlayerList, RemoteClient},
    util::LocalRepeatingTimer,
    world::{
        chunk::{ChunkType, ChunkCoord}, events::ChunkUpdatedEvent, settings::Settings, BlockId, BlockRegistry,
        BlockResources, ChunkBoundaryCrossedEvent, Level, LevelLoadState,
    }, serialization::ChunkSaveFormat, physics::movement::Velocity,
};

use super::{ClientMessage, ServerMessage, UpdateEntityTransform, UpdateEntityVelocity};

pub const TICK_MS: u64 = 10;

pub struct NetServerPlugin;

impl Plugin for NetServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetServerPlugin {
            initialize_later: true,
        })
        .add_state::<ServerState>()
        .add_systems(
            Update,
            create_server.run_if(resource_exists::<ServerConfig>().and_then(
                in_state(ServerState::NotStarted).and_then(in_state(LevelLoadState::Loaded)),
            )),
        )
        .add_systems(
            OnEnter(ServerState::Starting),
            start_listening.run_if(resource_exists::<ServerConfig>()),
        )
        .add_systems(
            Update,
            (
                handle_client_messages,
                handle_server_events,
                sync_entity_updates,
                push_chunks_chunk_updated,
                push_chunks_chunk_boundary_crossed,
                push_chunks_on_join
            )
                .run_if(in_state(ServerState::Started)),
        )
        .add_systems(
            Update,
            assign_server_player.run_if(
                not(resource_exists::<ServerPlayer>()).and_then(in_state(ServerState::Started)),
            ),
        );
    }
}

#[derive(Resource)]
pub struct ServerPlayer(pub PlayerInfo);

#[derive(Resource)]
pub struct ServerConfig {
    pub bind_addr: IpAddr,
    pub bind_port: u16,
}

#[derive(States, Default, Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub enum ServerState {
    #[default]
    NotStarted,
    Starting,
    Started,
}

//marker to sync entity's position with all clients
#[derive(Component)]
pub struct SyncPosition;

//marker to sync entity's velocity with all clients
#[derive(Component)]
pub struct SyncVelocity;

fn handle_client_messages(
    mut server: ResMut<Server>,
    mut users: ResMut<PlayerList>,
    server_player: Option<Res<ServerPlayer>>,
    mut commands: Commands,
    mut update_tf_writer: EventWriter<UpdateEntityTransform>,
    mut update_v_writer: EventWriter<UpdateEntityVelocity>,
    mut use_item_writer: EventWriter<UseItemEvent>,
    mut swing_item_writer: EventWriter<SwingItemEvent>,
    inventory_query: Query<&Inventory>,
    block_resources: Res<BlockResources>,
    item_resources: Res<ItemResources>,
    level: Res<Level>,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some(message) = endpoint.try_receive_message_from::<ClientMessage>(client_id) {
            match message {
                ClientMessage::Join { name } => {
                    handle_join(
                        client_id,
                        name,
                        level.get_spawn_point(),
                        &mut users,
                        server_player.as_ref().map(|s| s.0.clone()),
                        endpoint,
                        &mut commands,
                        &block_resources.registry,
                        &item_resources.registry,
                    );
                }
                ClientMessage::Disconnect {} => {
                    // We tell the server to disconnect this user
                    endpoint.disconnect_client(client_id).unwrap();
                    handle_disconnect(endpoint, &mut users, client_id, &mut commands);
                }
                ClientMessage::ChatMessage { message } => {
                    info!(
                        "Chat message | {:?}: {}",
                        users.infos.get(&client_id),
                        message
                    );
                    endpoint.try_send_group_message_on(
                        users.infos.keys(),
                        ChannelId::UnorderedReliable,
                        ServerMessage::ChatMessage { client_id, message },
                    );
                }
                ClientMessage::UpdatePosition {
                    transform,
                    velocity,
                } => {
                    if let Some(PlayerInfo {
                        username: _,
                        entity,
                    }) = users.infos.get(&client_id)
                    {
                        update_tf_writer.send(UpdateEntityTransform {
                            entity: *entity,
                            transform,
                        });
                        update_v_writer.send(UpdateEntityVelocity {
                            entity: *entity,
                            velocity,
                        });
                    } else {
                        warn!("Update position recieved for uninitialized client!");
                    }
                }
                ClientMessage::UseItem { tf, slot } => {
                    if let Some(PlayerInfo {
                        username: _,
                        entity
                    }) = users.infos.get(&client_id) {
                        if let Ok(Some(stack)) = inventory_query.get(*entity).map(|inv| inv.get(slot)) {
                            use_item_writer.send(UseItemEvent{ user: *entity, inventory_slot: Some(slot), stack, tf: tf.into()})
                        }
                    }
                },
                ClientMessage::SwingItem { tf, slot } => {
                    if let Some(PlayerInfo {
                        username: _,
                        entity
                    }) = users.infos.get(&client_id) {
                        if let Ok(Some(stack)) = inventory_query.get(*entity).map(|inv| inv.get(slot)) {
                            swing_item_writer.send(SwingItemEvent { user: *entity, inventory_slot: Some(slot), stack, tf: tf.into()})
                        }
                    }
                },
            }
        }
    }
}

fn handle_join(
    client_id: ClientId,
    username: String,
    spawn_point: Vec3,
    users: &mut PlayerList,
    server_player: Option<PlayerInfo>,
    endpoint: &mut Endpoint,
    commands: &mut Commands,
    block_registry: &BlockRegistry,
    item_registry: &ItemRegistry,
) {
    if users.infos.contains_key(&client_id) {
        warn!(
            "Received a Join from an already connected client: {}",
            client_id
        );
    } else {
        info!("{} connected", &username);
        info!("Server player: {:?}", server_player);
        let player_entity = commands.spawn(RemoteClient(Some(client_id))).id();
        let info = PlayerInfo {
            username,
            entity: player_entity,
        };
        users.infos.insert(client_id, info.clone());
        // Initialize this client with existing state
        endpoint
            .send_message(
                client_id,
                ServerMessage::InitClient {
                    client_id,
                    entity: player_entity,
                    spawn_point,
                    clients_online: PlayerList {
                        infos: users.infos.clone(),
                        server: server_player,
                    },
                    block_ids: block_registry.id_map.clone(),
                    item_ids: item_registry.id_map.clone(),
                },
            )
            .unwrap();
        // Broadcast the connection event
        endpoint
            .send_group_message(
                users
                    .infos
                    .keys()
                    .filter(|id| **id != client_id),
                ServerMessage::ClientConnected { client_id, info },
            )
            .unwrap();
    }
}

fn handle_server_events(
    mut connection_lost_events: EventReader<ConnectionLostEvent>,
    mut server: ResMut<Server>,
    mut users: ResMut<PlayerList>,
    mut commands: Commands,
) {
    // The server signals us about users that lost connection
    for client in connection_lost_events.read() {
        handle_disconnect(server.endpoint_mut(), &mut users, client.id, &mut commands);
    }
}

/// Shared disconnection behaviour, whether the client lost connection or asked to disconnect
fn handle_disconnect(
    endpoint: &mut Endpoint,
    users: &mut ResMut<PlayerList>,
    client_id: ClientId,
    commands: &mut Commands,
) {
    // Remove this user
    if let Some(info) = users.infos.remove(&client_id) {
        // Broadcast its deconnection

        endpoint
            .send_group_message(
                users.infos.keys(),
                ServerMessage::ClientDisconnected { client_id },
            )
            .unwrap();
        info!("{} disconnected", info.username);
        //TODO: i think it's okay to leak these, since the entities would be so small
        //other systems should listen for the removal of `RemoteClient` and do cleanup there.
        //could be useful even to keep these around (easy logging of all clients that have connected?)
        commands
            .entity(info.entity)
            .remove::<RemoteClient>()
            .insert(DisconnectedClient(Some(client_id)));
    } else {
        warn!(
            "Received a Disconnect from an unknown or disconnected client: {}",
            client_id
        )
    }
}

fn start_listening(
    mut server: ResMut<Server>,
    mut state: ResMut<NextState<ServerState>>,
    config: Res<ServerConfig>,
) {
    server
        .start_endpoint(
            ServerConfiguration::from_ip(config.bind_addr, config.bind_port),
            CertificateRetrievalMode::GenerateSelfSigned {
                server_hostname: "127.0.0.1".to_string(),
            },
        )
        .unwrap();
    state.set(ServerState::Started);
    info!(
        "Starting server on {} port {}",
        config.bind_addr, config.bind_port
    );
}

fn create_server(mut commands: Commands, mut state: ResMut<NextState<ServerState>>) {
    commands.init_resource::<Server>();
    state.set(ServerState::Starting);
    info!("Created server!");
}

fn sync_entity_updates(
    mut timer: Local<LocalRepeatingTimer<TICK_MS>>,
    time: Res<Time>,
    server: Res<Server>,
    clients: Res<PlayerList>,
    tfs: Query<(Entity, &Transform), With<SyncPosition>>,
    vs: Query<(Entity, &Velocity), With<SyncVelocity>>,
) {
    timer.tick(time.delta());
    if !timer.just_finished() {
        return;
    }
    //first bulk send all non player entities that we need to update
    //TODO: optimize to send only the ones in loaded chunks near each player
    let transforms: Vec<UpdateEntityTransform> = tfs
        .iter()
        .map(|(e, tf)| UpdateEntityTransform {
            entity: e,
            transform: *tf,
        })
        .collect();
    let velocities = vs
        .iter()
        .map(|(e, v)| UpdateEntityVelocity {
            entity: e,
            velocity: v.0,
        })
        .collect();
    if let Err(e) = server.endpoint().send_group_message_on(
        clients.infos.keys(),
        ChannelId::Unreliable,
        ServerMessage::UpdateEntities {
            transforms,
            velocities,
        },
    ) {
        error!("{}", e);
    }
}

fn assign_server_player(
    mut commands: Commands,
    local_player: Query<Entity, With<LocalPlayer>>,
    mut players: ResMut<PlayerList>,
) {
    if let Ok(entity) = local_player.get_single() {
        let info = PlayerInfo {
            username: "host".into(),
            entity,
        };
        commands.insert_resource(ServerPlayer(info.clone()));
        players.server = Some(info);
    }
}

fn send_chunk(
    coord: ChunkCoord,
    client_id: ClientId,
    level: &Level,
    server: &Server,
    id_query: &Query<&BlockId>
) {
    if let Some(r) = level.get_chunk(coord) {
        if let ChunkType::Full(c) = r.value() {
            if let Err(e) = server.endpoint().send_message(
                client_id,
                ServerMessage::Chunk {
                    chunk: ChunkSaveFormat::palette_ids_only_no_map((c.position, &c.blocks), id_query),
                },
            ) {
                error!("{}", e);
            }
        }
    }
}

fn push_chunks_on_join(
    remotes: Query<(&RemoteClient, &GlobalTransform), Added<GlobalTransform>>,
    server: Res<Server>,
    level: Res<Level>,
    settings: Res<Settings>,
    id_query: Query<&BlockId>,
) {
    for (RemoteClient(id_opt), tf) in remotes.iter() {
        if let Some(id) = id_opt {
            let coord: ChunkCoord = tf.translation().into();
            settings.player_loader.for_each_chunk(|offset| {
                info!("sending (join) chunk {:?}", offset+coord);
                send_chunk(offset+coord, *id, &level, &server, &id_query);
            })
        }
    }
}

//covers if a player crosses a chunk boundary and reaches already loaded chunks
fn push_chunks_chunk_boundary_crossed(
    remotes: Query<&RemoteClient>,
    mut crossed_reader: EventReader<ChunkBoundaryCrossedEvent>,
    server: Res<Server>,
    level: Res<Level>,
    settings: Res<Settings>,
    id_query: Query<&BlockId>,
) {
    let mut diff = HashSet::new();
    for ChunkBoundaryCrossedEvent {
        entity,
        old_position,
        new_position,
    } in crossed_reader.read()
    {
        if let Ok(RemoteClient(Some(id))) = remotes.get(*entity) {
            settings.player_loader.for_each_chunk(|offset| {
                diff.insert(offset + *new_position);
            });
            settings.player_loader.for_each_chunk(|offset| {
                diff.remove(&(offset + *old_position));
            });
            for coord in diff.drain() {
                info!("sending (crossed boundary) chunk {:?}", coord);
                send_chunk(coord, *id, &level, &server, &id_query);
            }
        }
    }
}

//covers if chunks are loaded or updated inside a player's sphere of influence
fn push_chunks_chunk_updated(
    mut reader: EventReader<ChunkUpdatedEvent>,
    remotes: Query<(&GlobalTransform, &RemoteClient)>,
    server: Res<Server>,
    level: Res<Level>,
    settings: Res<Settings>,
    id_query: Query<&BlockId>,
) {
    //TODO: spatially partition players so we don't have to check every player for every chunk
    for ChunkUpdatedEvent { coord } in reader.read() {
        info!("Chunk updated at {:?}", coord);
        for (tf, remote) in remotes.iter() {
            if !settings
                .player_loader
                .chunk_in_range(tf.translation().into(), *coord)
            {
                continue;
            }
            if let Some(id) = remote.0 {
                info!("sending (updated) chunk {:?}", coord);
                send_chunk(*coord, id, &level, &server, &id_query);
            }
        }
    }
}
