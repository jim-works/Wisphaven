use std::{hash::Hash, net::IpAddr, thread::sleep, time::Duration};

use bevy::{app::AppExit, prelude::*, utils::HashMap};
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use lightyear::{prelude::client::ClientCommands, shared::events::components::MessageEvent};
use rand::Rng;
use rand_distr::Alphanumeric;

use crate::{
    actors::{LocalPlayer, LocalPlayerSpawnedEvent},
    items::{ItemId, ItemResources},
    serialization::state::GameLoadState,
    world::{events::ChunkUpdatedEvent, BlockId, BlockResources, Level, LevelData},
    GameState,
};

use super::protocol::{ClientInfoMessage, OrderedReliable};
use super::{
    protocol::PlayerListMessage, ClientMessage, DisconnectedClient, NetworkType, PlayerInfo,
    PlayerList, RemoteClient, ServerMessage, UpdateEntityTransform, UpdateEntityVelocity,
};

pub(crate) struct ClientPlugin {
    pub(crate) network_type: NetworkType,
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ClientState>()
            .enable_state_scoped_entities::<ClientState>()
            .add_systems(OnEnter(GameState::Game), connect)
            .add_systems(OnEnter(NetworkingState::Connected), on_connected)
            .add_systems(Update, on_server_ready);

        // .add_systems(
        //     Update,
        //     create_client.run_if(
        //         resource_exists::<ClientConfig>
        //             .and(in_state(ClientState::NotStarted))
        //             .and(resource_exists::<Level>),
        //     ),
        // )
        // .add_systems(
        //     OnEnter(ClientState::Starting),
        //     start_listening.run_if(resource_exists::<QuinnetClient>),
        // )
        // .add_systems(
        //     Update,
        //     (
        //         handle_server_messages,
        //         handle_client_events,
        //         map_local_player,
        //     )
        //         .run_if(
        //             resource_exists::<QuinnetClient>
        //                 .and(resource_exists::<LocalClient>)
        //                 .and(in_state(ClientState::Started).or(in_state(ClientState::Ready)))
        //                 .and(in_state(GameLoadState::Done)),
        //         ),
        // )
        // // CoreSet::PostUpdate so that AppExit events generated in the previous stage are available
        // .add_systems(
        //     PostUpdate,
        //     on_app_exit.run_if(resource_exists::<QuinnetClient>),
        // );
    }
}

#[derive(Resource, Default)]
pub struct NetworkMap<T> {
    local_to_remote: HashMap<T, T>,
    remote_to_local: HashMap<T, T>,
}

impl<T> NetworkMap<T>
where
    T: Eq + Hash + Clone,
{
    //clones local and remote since we maintain two maps
    pub fn insert(&mut self, local: T, remote: T) {
        self.local_to_remote.insert(local.clone(), remote.clone());
        self.remote_to_local.insert(remote, local);
    }
    //returns the local entity corresponding to `remote_entity` if it exists
    pub fn remove_remote(&mut self, remote: &T) -> Option<T> {
        let local = self.remote_to_local.remove(remote);
        if let Some(ref l) = local {
            self.local_to_remote.remove(l);
        }
        local
    }
    pub fn local_to_remote(&self) -> &HashMap<T, T> {
        &self.local_to_remote
    }
    pub fn remote_to_local(&self) -> &HashMap<T, T> {
        &self.remote_to_local
    }
}
#[derive(States, Default, Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub enum ClientState {
    #[default]
    NotStarted,
    Started,
    //recieved initialization message from server
    Ready,
}

fn connect(mut commands: Commands) {
    info!("connecting client...");
    commands.connect_client();
}

fn on_connected(mut conn: ResMut<ClientConnectionManager>) {
    info!("On connected running");
    if let Err(e) =
        conn.send_message::<OrderedReliable, ClientInfoMessage>(&mut ClientInfoMessage {
            name: "jimbob".into(),
        })
    {
        error!("Error sending name: {:?}", e);
    }
}

fn on_server_ready(
    mut state: ResMut<NextState<ClientState>>,
    mut messages: EventReader<MessageEvent<PlayerListMessage>>,
) {
    for MessageEvent { message, .. } in messages.read() {
        info!(
            "There are {} players online: {:?}",
            message.name.len(),
            message.name
        );
        state.set(ClientState::Ready);
    }
}

// fn handle_server_messages(
//     mut users: ResMut<PlayerList>,
//     mut client: ResMut<QuinnetClient>,
//     mut state: ResMut<NextState<ClientState>>,
//     mut local_client: ResMut<LocalClient>,
//     mut entity_map: ResMut<LocalEntityMap>,
//     mut commands: Commands,
//     mut update_tf_writer: EventWriter<UpdateEntityTransform>,
//     mut update_v_writer: EventWriter<UpdateEntityVelocity>,
//     block_resources: Res<BlockResources>,
//     item_resources: Res<ItemResources>,
//     block_id_map: Option<Res<LocalMap<BlockId>>>,
//     level: Option<Res<Level>>,
//     mut chunk_update_writer: EventWriter<ChunkUpdatedEvent>,
// ) {
//     while let Some((_, message)) = client
//         .connection_mut()
//         .try_receive_message::<ServerMessage>()
//     {
//         match message {
//             ServerMessage::ClientConnected { client_id, info } => {
//                 info!("{} joined", info.username);
//                 setup_remote_player(&info, Some(client_id), &mut commands, &mut entity_map);
//                 users.infos.insert(client_id, info);
//             }
//             ServerMessage::ClientDisconnected { client_id } => {
//                 if let Some(info) = users.infos.remove(&client_id) {
//                     println!("{} left", info.username);
//                     handle_disconnect(&info, Some(client_id), &mut commands, &mut entity_map);
//                 } else {
//                     warn!("ClientDisconnected for an unknown client_id: {}", client_id)
//                 }
//             }
//             ServerMessage::ChatMessage { client_id, message } => {
//                 if let Some(id) = local_client.id {
//                     if let Some(info) = users.infos.get(&client_id) {
//                         if client_id != id {
//                             println!("{}: {}", info.username, message);
//                         }
//                     } else {
//                         warn!("Chat message from an unknown client_id: {}", client_id)
//                     }
//                 }
//             }
//             ServerMessage::InitClient {
//                 client_id: my_client_id,
//                 entity,
//                 spawn_point,
//                 clients_online,
//                 mut block_ids,
//                 mut item_ids,
//             } => {
//                 local_client.id = Some(my_client_id);
//                 local_client.server_entity = Some(entity);
//                 local_client.spawn_point = spawn_point;
//                 info!("Recieved initialization message: there are {} players online. ({} blocks and {} items)", clients_online.infos.len() + if clients_online.server.is_some() {1} else {0}, block_ids.len(), item_ids.len());
//                 info!("Players online: ");
//                 for (client_id, info) in clients_online.infos.iter() {
//                     if *client_id != my_client_id {
//                         setup_remote_player(info, Some(*client_id), &mut commands, &mut entity_map);
//                     }
//                     info!("username: {}", info.username);
//                 }
//                 if let Some(ref info) = clients_online.server {
//                     setup_remote_player(info, None, &mut commands, &mut entity_map);
//                 }
//                 *users = clients_online;
//                 //setup id maps, since name -> id mappings are not consistent across network boundary
//                 let mut block_id_map: LocalMap<BlockId> = LocalMap::default();
//                 for (name, id) in block_ids.drain() {
//                     block_id_map.insert(block_resources.registry.get_id(&name), id);
//                 }
//                 let mut item_id_map: LocalMap<ItemId> = LocalMap::default();
//                 for (name, id) in item_ids.drain() {
//                     item_id_map.insert(item_resources.registry.get_id(&name), id);
//                 }
//                 commands.insert_resource(block_id_map);
//                 commands.insert_resource(item_id_map);
//                 info!("Client recv InitClient");
//                 state.set(ClientState::Ready);
//             }
//             ServerMessage::UpdateEntities {
//                 transforms,
//                 velocities,
//             } => {
//                 for UpdateEntityTransform { entity, transform } in transforms {
//                     if let Some(local) = entity_map.remote_to_local().get(&entity) {
//                         update_tf_writer.send(UpdateEntityTransform {
//                             entity: *local,
//                             transform,
//                         });
//                     } else {
//                         warn!("Recv UpdateEntityTransform message for unknown entity!");
//                     }
//                 }
//                 for UpdateEntityVelocity { entity, velocity } in velocities {
//                     if let Some(local) = entity_map.remote_to_local().get(&entity) {
//                         update_v_writer.send(UpdateEntityVelocity {
//                             entity: *local,
//                             velocity,
//                         });
//                     } else {
//                         warn!("Recv UpdateEntityVelocity message for unknown entity!");
//                     }
//                 }
//             }
//             ServerMessage::Chunk { mut chunk } => {
//                 info!("recv chunk at {:?}", chunk.position);
//                 //TODO: discard chunks that are too far away
//                 if let Some(ref id_map) = block_id_map {
//                     match level {
//                         Some(ref level) => {
//                             for (val, _) in chunk.data.iter_mut() {
//                                 *val = *id_map.remote_to_local().get(val).unwrap();
//                             }
//                             let coord = chunk.position;
//                             let id = level.overwrite_or_spawn_chunk(
//                                 coord,
//                                 chunk,
//                                 &mut commands,
//                                 &block_resources.registry,
//                             );
//                             LevelData::update_chunk_only::<false>(
//                                 id,
//                                 coord,
//                                 &mut commands,
//                                 &mut chunk_update_writer,
//                             );
//                             level.update_chunk_neighbors_only(
//                                 coord,
//                                 &mut commands,
//                                 &mut chunk_update_writer,
//                             );
//                         }
//                         None => {
//                             warn!("recv chunk before level is ready")
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

// //only sets up in regard to the world
// //doesn't add to Clients hashmap, do that separately if needed.
// fn setup_remote_player(
//     remote: &PlayerInfo,
//     remote_id: Option<ClientId>,
//     commands: &mut Commands,
//     entity_map: &mut LocalEntityMap,
// ) {
//     let local_entity = commands
//         .spawn((
//             StateScoped(GameState::Game),
//             RemoteClient(remote_id),
//             Name::new(remote.username.clone()),
//         ))
//         .id();
//     entity_map.insert(local_entity, remote.entity);
//     info!("Setup remote player: {}", remote.username);
// }

// fn setup_local_player(
//     mut reader: EventReader<LocalPlayerSpawnedEvent>,
//     local_client: Res<LocalClient>,
//     mut entity_map: ResMut<LocalEntityMap>,
// ) {
//     for LocalPlayerSpawnedEvent(local_entity) in reader.read() {
//         //we recv our entity from the server before spawning in the local player
//         entity_map.insert(*local_entity, local_client.server_entity.unwrap());
//     }
// }

// fn handle_disconnect(
//     remote: &PlayerInfo,
//     remote_id: Option<ClientId>,
//     commands: &mut Commands,
//     entity_map: &mut LocalEntityMap,
// ) {
//     if let Some(local) = entity_map.remove_remote(remote.entity) {
//         commands
//             .entity(local)
//             .remove::<RemoteClient>()
//             .insert(DisconnectedClient(remote_id));
//     }
// }

// fn handle_client_events(
//     mut connection_events: EventReader<ConnectionEvent>,
//     client: ResMut<QuinnetClient>,
// ) {
//     if !connection_events.is_empty() {
//         // We are connected
//         let username: String = rand::thread_rng()
//             .sample_iter(&Alphanumeric)
//             .take(7)
//             .map(char::from)
//             .collect();

//         println!("--- Joining with name: {}", username);

//         client
//             .connection()
//             .send_message(ClientMessage::Join { name: username })
//             .unwrap();

//         connection_events.clear();
//     }
// }
// fn start_listening(
//     mut client: ResMut<QuinnetClient>,
//     mut state: ResMut<NextState<ClientState>>,
//     local_client: Res<LocalClient>,
//     channels: Res<ChannelsConfig>,
// ) {
//     client
//         .open_connection(
//             ClientEndpointConfiguration::from_ips(
//                 local_client.server_ip,
//                 local_client.server_port,
//                 local_client.local_ip,
//                 local_client.local_port,
//             ),
//             CertificateVerificationMode::SkipVerification,
//             channels.config.clone(),
//         )
//         .unwrap();
//     state.set(ClientState::Started);
//     info!("Client started!");
// }

// fn create_client(
//     mut commands: Commands,
//     config: Res<ClientConfig>,
//     mut state: ResMut<NextState<ClientState>>,
// ) {
//     commands.init_resource::<QuinnetClient>();
//     commands.insert_resource(LocalClient {
//         id: None,
//         server_entity: None,
//         spawn_point: Vec3::default(),
//         server_ip: config.server_ip,
//         server_port: config.server_port,
//         local_ip: config.local_ip,
//         local_port: config.local_port,
//     });
//     commands.insert_resource(LocalEntityMap::default());
//     state.set(ClientState::Starting);
//     info!("Creating client");
// }

// fn map_local_player(
//     client: Res<LocalClient>,
//     mut id_map: ResMut<LocalEntityMap>,
//     player: Query<Entity, Added<LocalPlayer>>,
// ) {
//     for player in player.iter() {
//         id_map.insert(
//             player,
//             client
//                 .server_entity
//                 .expect("Server entity should be set up before local player is spawned!"),
//         );
//         info!("Mapped local player");
//     }
// }
