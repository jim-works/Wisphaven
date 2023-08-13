use bevy::{prelude::*, utils::HashMap};

use bevy_quinnet::shared::ClientId;
use bevy_rapier3d::prelude::Velocity;
use serde::{Deserialize, Serialize};

pub mod client;
pub mod server;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((server::NetServerPlugin, client::NetClientPlugin))
            .add_systems(PostUpdate, (process_transform_updates, process_velocity_updates))
            .add_event::<UpdateEntityTransform>()
            .add_event::<UpdateEntityVelocity>()
            .insert_resource(PlayerList::default());
    }
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerList {
    pub infos: HashMap<ClientId, PlayerInfo>,
    pub server: Option<PlayerInfo>
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
        clients_online: PlayerList,
    },
    UpdateEntities {
        transforms: Vec<UpdateEntityTransform>,
        velocities: Vec<UpdateEntityVelocity>
    },
}

//recv from over the network
#[derive(Event, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct UpdateEntityTransform {
    pub entity: Entity,
    pub transform: Transform
}

//recv from over the network
#[derive(Event, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct UpdateEntityVelocity {
    pub entity: Entity,
    pub velocity: Vec3
}

fn process_transform_updates (
    mut reader: EventReader<UpdateEntityTransform>,
    mut query: Query<&mut Transform>
) {
    for UpdateEntityTransform { entity, transform } in reader.iter() {
        if let Ok(mut tf) = query.get_mut(*entity) {
            *tf = *transform
        } else {
            warn!("Recv UpdateEntityTransform for entity that doesn't have a transform!");
        }
    }
}

fn process_velocity_updates (
    mut reader: EventReader<UpdateEntityVelocity>,
    mut query: Query<&mut Velocity>
) {
    for UpdateEntityVelocity { entity, velocity } in reader.iter() {
        if let Ok(mut v) = query.get_mut(*entity) {
            v.linvel = *velocity
        } else {
            warn!("Recv UpdateEntityVelocity for entity that doesn't have a velocity!");
        }
    }
}