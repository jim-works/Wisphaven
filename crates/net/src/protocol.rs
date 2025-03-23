use bevy::{ecs::system::SystemParam, prelude::*};
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

pub(crate) struct ProtocolPlugin;

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
    }
}

#[derive(Channel)]
pub(crate) struct OrderedReliable;
#[derive(Channel)]
pub(crate) struct UnorderedReliable;

#[derive(Channel)]
pub(crate) struct UnorderedUnreliable;

// client sends on connect
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct ClientInfoMessage {
    pub(crate) name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PlayerListMessage {
    pub name: Vec<String>,
}

// server sends on client join - recieving this moves the client into the ready state
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct InitMessage {
    pub(crate) block_map: BlockNameIdMap,
    pub(crate) item_map: ItemNameIdMap,
    pub(crate) actor_map: ActorNameIdMap,
    pub(crate) projectile_map: ProjectileNameIdMap,
}

#[derive(SystemParam)]
pub(crate) struct InitMessageSystemParam<'w> {
    pub(crate) block_resources: Res<'w, BlockResources>,
    pub(crate) item_resources: Res<'w, ItemResources>,
    pub(crate) actor_resources: Res<'w, ActorResources>,
    pub(crate) projectile_registry: Res<'w, ProjectileRegistry>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ChunkMessage {
    pub(crate) chunk: ChunkSaveFormat,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RequestChunksMessage {
    pub(crate) coords: Vec<ChunkCoord>,
}
