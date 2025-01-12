use bevy::prelude::*;
use client::{ComponentSyncMode, LerpFn};
use lightyear::{prelude::*, utils::bevy::TransformLinearInterpolation};
use serde::{Deserialize, Serialize};

use crate::physics::movement::{Acceleration, Velocity};

use super::RemoteClient;

pub(crate) struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.register_message::<ClientInfoMessage>(ChannelDirection::ClientToServer);
        app.register_message::<PlayerListMessage>(ChannelDirection::ServerToClient);

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
pub struct OrderedReliable;
#[derive(Channel)]
pub struct UnorderedReliable;

#[derive(Channel)]
pub struct UnorderedUnreliable;

// client sends on connect
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct ClientInfoMessage {
    pub name: String,
}

// server sends on client join
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PlayerListMessage {
    pub name: Vec<String>,
}
