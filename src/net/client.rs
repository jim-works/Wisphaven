use std::{net::IpAddr, thread::sleep, time::Duration};

use bevy::{app::AppExit, ecs::entity::EntityMap, prelude::*};
use bevy_quinnet::{
    client::{
        certificate::CertificateVerificationMode,
        connection::{ConnectionConfiguration, ConnectionEvent},
        Client, QuinnetClientPlugin,
    },
    shared::ClientId,
};
use rand::Rng;
use rand_distr::Alphanumeric;

use crate::actors::LocalPlayerSpawnedEvent;

use super::{
    ClientMessage, DisconnectedClient, PlayerInfo, PlayerList, RemoteClient, ServerMessage,
    UpdateEntityTransform, UpdateEntityVelocity,
};

pub struct NetClientPlugin;

impl Plugin for NetClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetClientPlugin {
            initialize_later: true,
        })
        .add_state::<ClientState>()
        .add_event::<StartClientEvent>()
        .add_systems(OnEnter(ClientState::NotStarted), create_client)
        .add_systems(
            OnEnter(ClientState::Starting),
            start_listening.run_if(resource_exists::<Client>()),
        )
        .add_systems(
            Update,
            (handle_server_messages, handle_client_events).run_if(
                resource_exists::<Client>()
                    .and_then(resource_exists::<LocalClient>())
                    .and_then(in_state(ClientState::Started)),
            ),
        )
        // CoreSet::PostUpdate so that AppExit events generated in the previous stage are available
        .add_systems(PostUpdate, on_app_exit.run_if(resource_exists::<Client>()));
    }
}

#[derive(Resource)]
pub struct LocalClient {
    id: Option<ClientId>,
    server_entity: Option<Entity>,
    server_ip: IpAddr,
    server_port: u16,
    local_ip: IpAddr,
    local_port: u16,
}

#[derive(Resource)]
pub struct LocalEntityMap {
    local_to_remote: EntityMap,
    remote_to_local: EntityMap,
}

impl LocalEntityMap {
    pub fn insert(&mut self, local_entity: Entity, remote_entity: Entity) {
        self.local_to_remote.insert(local_entity, remote_entity);
        self.remote_to_local.insert(remote_entity, local_entity);
    }
    //returns the local entity corresponding to `remote_entity` if it exists
    pub fn remove_remote(&mut self, remote_entity: Entity) -> Option<Entity> {
        let local = self.remote_to_local.remove(remote_entity);
        if let Some(l) = local {
            self.local_to_remote.remove(l);
        }
        local
    }
    pub fn local_to_remote(&self) -> &EntityMap {
        &self.local_to_remote
    }
    pub fn remote_to_local(&self) -> &EntityMap {
        &self.remote_to_local
    }
}

impl Default for LocalEntityMap {
    fn default() -> Self {
        Self {
            local_to_remote: Default::default(),
            remote_to_local: Default::default(),
        }
    }
}

#[derive(Event)]
pub struct StartClientEvent {
    pub server_ip: IpAddr,
    pub server_port: u16,
    pub local_ip: IpAddr,
    pub local_port: u16,
}

#[derive(States, Default, Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub enum ClientState {
    #[default]
    NotStarted,
    Starting,
    Started,
    //recieved initialization message from server
    Ready,
}

pub fn on_app_exit(app_exit_events: EventReader<AppExit>, client: Res<Client>) {
    if !app_exit_events.is_empty() {
        client
            .connection()
            .send_message(ClientMessage::Disconnect {})
            .unwrap();
        // TODO Clean: event to let the async client send his last messages.
        sleep(Duration::from_secs_f32(0.1));
        info!("cleaned up client");
    }
}

fn handle_server_messages(
    mut users: ResMut<PlayerList>,
    mut client: ResMut<Client>,
    mut state: ResMut<NextState<ClientState>>,
    mut local_client: ResMut<LocalClient>,
    mut entity_map: ResMut<LocalEntityMap>,
    mut commands: Commands,
    mut update_tf_writer: EventWriter<UpdateEntityTransform>,
    mut update_v_writer: EventWriter<UpdateEntityVelocity>,
) {
    while let Some(message) = client
        .connection_mut()
        .try_receive_message::<ServerMessage>()
    {
        match message {
            ServerMessage::ClientConnected { client_id, info } => {
                info!("{} joined", info.username);
                setup_remote_player(&info, Some(client_id), &mut commands, &mut entity_map);
                users.infos.insert(client_id, info);
            }
            ServerMessage::ClientDisconnected { client_id } => {
                if let Some(info) = users.infos.remove(&client_id) {
                    println!("{} left", info.username);
                    handle_disconnect(&info, Some(client_id), &mut commands, &mut entity_map);
                } else {
                    warn!("ClientDisconnected for an unknown client_id: {}", client_id)
                }
            }
            ServerMessage::ChatMessage { client_id, message } => {
                if let Some(id) = local_client.id {
                    if let Some(info) = users.infos.get(&client_id) {
                        if client_id != id {
                            println!("{}: {}", info.username, message);
                        }
                    } else {
                        warn!("Chat message from an unknown client_id: {}", client_id)
                    }
                }
            }
            ServerMessage::InitClient {
                client_id: my_client_id,
                entity,
                clients_online,
            } => {
                local_client.id = Some(my_client_id);
                local_client.server_entity = Some(entity);
                for (client_id, info) in clients_online.infos.iter() {
                    if *client_id != my_client_id {
                        setup_remote_player(info, Some(*client_id), &mut commands, &mut entity_map);
                    }
                }
                if let Some(ref info) = clients_online.server {
                    setup_remote_player(info, None, &mut commands, &mut entity_map);
                }
                *users = clients_online;
                info!("Client recv InitClient");
                state.set(ClientState::Ready);
            }
            ServerMessage::UpdateEntities {
                transforms,
                velocities,
            } => {
                for UpdateEntityTransform { entity, transform } in transforms {
                    if let Some(local) = entity_map.remote_to_local().get(entity) {
                        update_tf_writer.send(UpdateEntityTransform {
                            entity: local,
                            transform,
                        });
                    } else {
                        warn!("Recv UpdateEntityTransform message for unknown entity!");
                    }
                }
                for UpdateEntityVelocity { entity, velocity } in velocities {
                    if let Some(local) = entity_map.remote_to_local().get(entity) {
                        update_v_writer.send(UpdateEntityVelocity {
                            entity: local,
                            velocity,
                        });
                    } else {
                        warn!("Recv UpdateEntityVelocity message for unknown entity!");
                    }
                }
            }
        }
    }
}

//only sets up in regard to the world
//doesn't add to Clients hashmap, do that separately if needed.
fn setup_remote_player(
    remote: &PlayerInfo,
    remote_id: Option<ClientId>,
    commands: &mut Commands,
    entity_map: &mut LocalEntityMap,
) {
    let local_entity = commands.spawn(RemoteClient(remote_id)).id();
    entity_map.insert(local_entity, remote.entity);
    info!("Setup remote player: {}", remote.username);
}

fn setup_local_player(
    mut reader: EventReader<LocalPlayerSpawnedEvent>,
    local_client: Res<LocalClient>,
    mut entity_map: ResMut<LocalEntityMap>,
) {
    for LocalPlayerSpawnedEvent(local_entity) in reader.iter() {
        //we recv our entity from the server before spawning in the local player
        entity_map.insert(*local_entity, local_client.server_entity.unwrap());
    }
}

fn handle_disconnect(
    remote: &PlayerInfo,
    remote_id: Option<ClientId>,
    commands: &mut Commands,
    entity_map: &mut LocalEntityMap,
) {
    if let Some(local) = entity_map.remove_remote(remote.entity) {
        commands
            .entity(local)
            .remove::<RemoteClient>()
            .insert(DisconnectedClient(remote_id));
    }
}

fn handle_client_events(
    mut connection_events: EventReader<ConnectionEvent>,
    client: ResMut<Client>,
) {
    if !connection_events.is_empty() {
        // We are connected
        let username: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();

        println!("--- Joining with name: {}", username);

        client
            .connection()
            .send_message(ClientMessage::Join { name: username })
            .unwrap();

        connection_events.clear();
    }
}
fn start_listening(
    mut client: ResMut<Client>,
    mut state: ResMut<NextState<ClientState>>,
    local_client: Res<LocalClient>,
) {
    client
        .open_connection(
            ConnectionConfiguration::from_ips(
                local_client.server_ip,
                local_client.server_port,
                local_client.local_ip,
                local_client.local_port,
            ),
            CertificateVerificationMode::SkipVerification,
        )
        .unwrap();
    state.set(ClientState::Started);
}

fn create_client(
    mut commands: Commands,
    mut reader: EventReader<StartClientEvent>,
    mut state: ResMut<NextState<ClientState>>,
) {
    for event in reader.iter() {
        commands.init_resource::<Client>();
        commands.insert_resource(LocalClient {
            id: None,
            server_entity: None,
            server_ip: event.server_ip,
            server_port: event.server_port,
            local_ip: event.local_ip,
            local_port: event.local_port,
        });
        commands.insert_resource(LocalEntityMap::default());
        state.set(ClientState::Starting);
    }
}
