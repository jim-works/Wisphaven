use std::net::Ipv4Addr;

use bevy::{prelude::*, utils::HashMap};

use bevy_quinnet::shared::ClientId;
use serde::{Deserialize, Serialize};

use self::{client::StartClientEvent, server::StartServerEvent};

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
struct Clients {
    names: HashMap<ClientId, String>,
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
        username: String,
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
        usernames: HashMap<ClientId, String>,
    },
}
