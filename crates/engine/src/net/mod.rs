use bevy::{prelude::*, utils::HashMap};

use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    actors::LocalPlayer, items::ItemNameIdMap, physics::movement::Velocity,
    serialization::ChunkSaveFormat, world::BlockNameIdMap,
};

use self::{client::ClientState, server::ServerState};

pub mod client;
pub mod config;
mod protocol;
pub mod server;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (process_transform_updates, process_velocity_updates),
        )
        .add_event::<UpdateEntityTransform>()
        .add_event::<UpdateEntityVelocity>()
        .init_state::<NetworkType>()
        .enable_state_scoped_entities::<NetworkType>()
        .insert_resource(PlayerList::default());
    }
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerList {
    pub infos: HashMap<ClientId, PlayerInfo>,
}

impl PlayerList {
    pub fn get(&self, id: &ClientId) -> Option<&PlayerInfo> {
        self.infos.get(id)
    }
}

//if none, belongs to server
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
pub struct RemoteClient(pub ClientId);

//if none, belongs to server
#[derive(Component)]
pub struct DisconnectedClient(pub ClientId);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerInfo {
    pub username: String,
    pub entity: Entity,
}

#[derive(States, Hash, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub enum NetworkType {
    #[default]
    Inactive,
    Server,
    Client,
    Host,
}

impl NetworkType {
    pub fn is_server(self) -> bool {
        matches!(self, NetworkType::Server | NetworkType::Host)
    }
    pub fn is_client(self) -> bool {
        matches!(self, NetworkType::Client | NetworkType::Host)
    }
    pub fn to_network_mode(self) -> Mode {
        match self {
            NetworkType::Host => Mode::HostServer,
            _ => Mode::Separate,
        }
    }
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
    },
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
        chunk: ChunkSaveFormat,
    },
}

pub fn network_ready() -> impl Condition<()> {
    in_state(NetworkType::Host)
        .and(in_state(ServerState::Active))
        .and(in_state(ServerState::Active))
        .or(in_state(NetworkType::Server).and(in_state(ServerState::Active)))
        .or(in_state(NetworkType::Client).and(in_state(ClientState::Ready)))
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
    local_player_query: Query<&LocalPlayer>,
) {
    const LOCAL_PLAYER_UPDATE_SQR_DIST: f32 = 1.0; //only update our local position if there's a desync with the server to avoid
                                                   //stuttery or frozen movement
    for UpdateEntityTransform { entity, transform } in reader.read() {
        if let Ok(mut tf) = query.get_mut(*entity) {
            if local_player_query.contains(*entity)
                && tf.translation.distance_squared(transform.translation)
                    < LOCAL_PLAYER_UPDATE_SQR_DIST
            {
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
    local_player_query: Query<&LocalPlayer>,
) {
    const LOCAL_PLAYER_UPDATE_SQR_DIST: f32 = 100.0; //only update our local position if there's a desync with the server to avoid
                                                     //stuttery or frozen movement
    for UpdateEntityVelocity { entity, velocity } in reader.read() {
        if let Ok(mut v) = query.get_mut(*entity) {
            if local_player_query.contains(*entity)
                && v.0.distance_squared(*velocity) < LOCAL_PLAYER_UPDATE_SQR_DIST
            {
                continue;
            }
            v.0 = *velocity
        } else {
            warn!("Recv UpdateEntityVelocity for entity that doesn't have a velocity!");
        }
    }
}
