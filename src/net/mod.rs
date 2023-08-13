use bevy::{prelude::*, utils::HashMap};

use bevy_quinnet::shared::ClientId;
use serde::{Deserialize, Serialize};

pub mod client;
pub mod server;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((server::NetServerPlugin, client::NetClientPlugin))
            .insert_resource(Clients::default());
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct Clients {
    pub infos: HashMap<ClientId, ClientConnectionInfo>,
}

#[derive(Component)]
pub struct RemoteClient(pub ClientId);

#[derive(Component)]
pub struct DisconnectedClient(pub ClientId);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientConnectionInfo {
    pub username: String,
    pub entity: Entity
}

// Messages from clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Join { name: String },
    Disconnect {},
    ChatMessage { message: String },
}

// Messages from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    ClientConnected {
        client_id: ClientId,
        info: ClientConnectionInfo,
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
        clients_online: HashMap<ClientId, ClientConnectionInfo>,
    },

}
