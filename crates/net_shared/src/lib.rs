use bevy::{ecs::system::SystemParam, prelude::*, utils::HashMap};
use client::{ComponentSyncMode, LerpFn};
use engine::{
    actors::{ActorNameIdMap, ActorResources},
    items::{ItemNameIdMap, ItemResources},
};
use interfaces::components::RemoteClient;
use lightyear::{prelude::*, utils::bevy::TransformLinearInterpolation};
use serde::{Deserialize, Serialize};

use actors::spawning::{ProjectileNameIdMap, ProjectileRegistry};
use physics::movement::{Acceleration, Velocity};
use world::{
    block::{BlockNameIdMap, BlockResources},
    chunk::{ChunkCoord, ChunkSaveFormat},
};

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.register_message::<ClientInfoMessage>(ChannelDirection::ClientToServer);
        app.register_message::<PlayerListMessage>(ChannelDirection::ServerToClient);
        app.register_message::<InitMessage>(ChannelDirection::ServerToClient);
        app.register_message::<ChunkMessage>(ChannelDirection::ServerToClient);
        app.register_message::<RequestChunksMessage>(ChannelDirection::ClientToServer);

        // components
        app.register_component::<RemoteClient>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once)
            .add_interpolation(ComponentSyncMode::Once);

        app.register_component::<Name>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Simple)
            .add_interpolation(ComponentSyncMode::Simple);

        // visual component, so needs interpolation
        app.register_component::<Transform>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation(ComponentSyncMode::Full)
            .add_interpolation_fn(TransformLinearInterpolation::lerp);

        // these aren't visual, so no need for interpolation
        app.register_component::<Velocity>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);
        app.register_component::<Acceleration>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        //channels
        app.add_channel::<OrderedReliable>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        });
        app.add_channel::<UnorderedReliable>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings::default()),
            ..default()
        });
        app.add_channel::<UnorderedUnreliable>(ChannelSettings {
            mode: ChannelMode::UnorderedUnreliable,
            ..default()
        });

        //resources
        app.init_resource::<PlayerList>();
    }
}

#[derive(Channel)]
pub struct OrderedReliable;
#[derive(Channel)]
pub struct UnorderedReliable;

#[derive(Channel)]
pub struct UnorderedUnreliable;

// client sends on connect
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ClientInfoMessage {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerListMessage {
    pub name: Vec<String>,
}

// server sends on client join - recieving this moves the client into the ready state
#[derive(Serialize, Deserialize, Default)]
pub struct InitMessage {
    pub block_map: BlockNameIdMap,
    pub item_map: ItemNameIdMap,
    pub actor_map: ActorNameIdMap,
    pub projectile_map: ProjectileNameIdMap,
}

#[derive(SystemParam)]
pub struct InitMessageSystemParam<'w> {
    pub block_resources: Res<'w, BlockResources>,
    pub item_resources: Res<'w, ItemResources>,
    pub actor_resources: Res<'w, ActorResources>,
    pub projectile_registry: Res<'w, ProjectileRegistry>,
}

#[derive(Serialize, Deserialize)]
pub struct ChunkMessage {
    pub chunk: ChunkSaveFormat,
}

#[derive(Serialize, Deserialize)]
pub struct RequestChunksMessage {
    pub coords: Vec<ChunkCoord>,
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
#[derive(Component)]
pub struct DisconnectedClient(pub ClientId);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerInfo {
    pub username: String,
    pub entity: Entity,
}
