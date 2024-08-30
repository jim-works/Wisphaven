use bevy::{prelude::*, utils::HashMap};

use bevy_quinnet::shared::ClientId;
use serde::{Deserialize, Serialize};

use crate::{items::ItemNameIdMap, world::BlockNameIdMap, actors::LocalPlayer, serialization::ChunkSaveFormat, physics::movement::Velocity};

use self::{client::ClientState, server::ServerState};

pub mod client;
pub mod server;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((server::NetServerPlugin, client::NetClientPlugin))
            .add_systems(
                PostUpdate,
                (process_transform_updates, process_velocity_updates),
            )
            .add_event::<UpdateEntityTransform>()
            .add_event::<UpdateEntityVelocity>()
            .add_state::<NetworkType>()
            .insert_resource(PlayerList::default());
    }
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerList {
    pub infos: HashMap<ClientId, PlayerInfo>,
    pub server: Option<PlayerInfo>,
}

impl PlayerList {
    //None for server's player
    pub fn get(&self, id: Option<ClientId>) -> Option<&PlayerInfo> {
        match id {
            Some(id) => self.infos.get(&id),
            None => self.server.as_ref(),
        }
    }
}

//if none, belongs to server
#[derive(Component)]
pub struct RemoteClient(pub Option<ClientId>);

//if none, belongs to server
#[derive(Component)]
pub struct DisconnectedClient(pub Option<ClientId>);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerInfo {
    pub username: String,
    pub entity: Entity,
}

#[derive(States, Hash, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub enum NetworkType {
    #[default]
    Inactive,
    Singleplayer,
    Server,
    Client,
}

// Messages from clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Join {
        name: String,
    },
    Disconnect {},
    ChatMessage {
        message: String,
    },
    UpdatePosition {
        transform: Transform,
        velocity: Vec3,
    },
    UseItem {
        tf: GlobalTransform,
        slot: usize,
    },
    SwingItem {
        tf: GlobalTransform,
        slot: usize,
    }
}

// Messages from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    ClientConnected {
        client_id: ClientId,
        info: PlayerInfo,
    },
    ClientDisconnected {
        client_id: ClientId,
    },
    ChatMessage {
        client_id: ClientId,
        message: String,
    },
    InitClient {
        client_id: ClientId,
        entity: Entity,
        spawn_point: Vec3,
        clients_online: PlayerList,
        block_ids: BlockNameIdMap,
        item_ids: ItemNameIdMap,
    },
    UpdateEntities {
        transforms: Vec<UpdateEntityTransform>,
        velocities: Vec<UpdateEntityVelocity>,
    },
    Chunk {
        chunk: ChunkSaveFormat
    }
}

pub fn network_ready() -> impl Condition<()> {
    in_state(NetworkType::Singleplayer)
        .or_else(in_state(NetworkType::Server).and_then(in_state(ServerState::Started)))
        .or_else(in_state(NetworkType::Client).and_then(in_state(ClientState::Ready)))
}

//recv from over the network
#[derive(Event, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct UpdateEntityTransform {
    pub entity: Entity,
    pub transform: Transform,
}

//recv from over the network
#[derive(Event, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct UpdateEntityVelocity {
    pub entity: Entity,
    pub velocity: Vec3,
}

fn process_transform_updates(
    mut reader: EventReader<UpdateEntityTransform>,
    mut query: Query<&mut Transform>,
    local_player_query: Query<&LocalPlayer>
) {
    const LOCAL_PLAYER_UPDATE_SQR_DIST: f32 = 1.0; //only update our local position if there's a desync with the server to avoid
                                                    //stuttery or frozen movement
    for UpdateEntityTransform { entity, transform } in reader.read() {
        if let Ok(mut tf) = query.get_mut(*entity) {
            if local_player_query.contains(*entity) && tf.translation.distance_squared(transform.translation) < LOCAL_PLAYER_UPDATE_SQR_DIST {
                continue;
            }
            *tf = *transform

        } else {
            warn!("Recv UpdateEntityTransform for entity that doesn't have a transform!");
        }
    }
}

fn process_velocity_updates(
    mut reader: EventReader<UpdateEntityVelocity>,
    mut query: Query<&mut Velocity>,
    local_player_query: Query<&LocalPlayer>
) {
    const LOCAL_PLAYER_UPDATE_SQR_DIST: f32 = 100.0; //only update our local position if there's a desync with the server to avoid
                                                    //stuttery or frozen movement
    for UpdateEntityVelocity { entity, velocity } in reader.read() {
        if let Ok(mut v) = query.get_mut(*entity) {
            if local_player_query.contains(*entity) && v.0.distance_squared(*velocity) < LOCAL_PLAYER_UPDATE_SQR_DIST {
                continue;
            }
            v.0 = *velocity
        } else {
            warn!("Recv UpdateEntityVelocity for entity that doesn't have a velocity!");
        }
    }
}
