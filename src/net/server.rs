//based on https://github.com/Henauxg/bevy_quinnet/blob/main/examples/chat/server.rs

use std::net::IpAddr;

use bevy::prelude::*;
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, ConnectionLostEvent, Endpoint, QuinnetServerPlugin,
        Server, ServerConfiguration,
    },
    shared::{channel::ChannelId, ClientId},
};
use bevy_rapier3d::prelude::Velocity;

use crate::{
    net::{DisconnectedClient, PlayerInfo, PlayerList, RemoteClient},
    world::LevelLoadState,
};

use super::{ClientMessage, ServerMessage, UpdateEntityTransform, UpdateEntityVelocity};

pub struct NetServerPlugin;

impl Plugin for NetServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetServerPlugin {
            initialize_later: true,
        })
        .add_state::<ServerState>()
        .add_systems(
            Update,
            create_server.run_if(
                in_state(ServerState::NotStarted).and_then(in_state(LevelLoadState::Loaded)),
            ),
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
            )
                .run_if(resource_exists::<Server>().and_then(in_state(ServerState::Started))),
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

//marker to sync entity's position and velocity with all clients
#[derive(Component)]
pub struct SyncPositionVelocity;

fn handle_client_messages(
    mut server: ResMut<Server>,
    mut users: ResMut<PlayerList>,
    server_player: Option<Res<ServerPlayer>>,
    mut commands: Commands,
    mut update_tf_writer: EventWriter<UpdateEntityTransform>,
    mut update_v_writer: EventWriter<UpdateEntityVelocity>,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some(message) = endpoint.try_receive_message_from::<ClientMessage>(client_id) {
            match message {
                ClientMessage::Join { name } => {
                    handle_join(
                        client_id,
                        name,
                        &mut users,
                        server_player.as_ref().map(|s| s.0.clone()),
                        endpoint,
                        &mut commands,
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
                        users.infos.keys().into_iter(),
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
            }
        }
    }
}

fn handle_join(
    client_id: ClientId,
    username: String,
    users: &mut PlayerList,
    server_player: Option<PlayerInfo>,
    endpoint: &mut Endpoint,
    commands: &mut Commands,
) {
    if users.infos.contains_key(&client_id) {
        warn!(
            "Received a Join from an already connected client: {}",
            client_id
        );
    } else {
        info!("{} connected", &username);
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
                    clients_online: PlayerList {
                        infos: users.infos.clone(),
                        server: server_player,
                    },
                },
            )
            .unwrap();
        // Broadcast the connection event
        endpoint
            .send_group_message(
                users
                    .infos
                    .keys()
                    .into_iter()
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
    for client in connection_lost_events.iter() {
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
                users.infos.keys().into_iter(),
                ServerMessage::ClientDisconnected {
                    client_id: client_id,
                },
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
    server: Res<Server>,
    clients: Res<PlayerList>,
    tfs: Query<(Entity, &Transform), With<SyncPositionVelocity>>,
    vs: Query<(Entity, &Velocity), With<SyncPositionVelocity>>,
) {
    let transforms = tfs
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
            velocity: v.linvel,
        })
        .collect();
    if let Err(e) = server.endpoint().send_group_message(
        clients.infos.keys().into_iter(),
        ServerMessage::UpdateEntities {
            transforms,
            velocities,
        },
    ) {
        error!("{}", e);
    }
}
