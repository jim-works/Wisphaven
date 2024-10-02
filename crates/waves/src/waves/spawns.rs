use std::sync::Arc;

use bevy::prelude::*;

use actors::spawning::{DefaultSpawnArgs, SpawnActorEvent};
use engine::actors::skeleton_pirate::SpawnSkeletonPirateEvent;
use util::SendEventCommand;

use super::SpawnAction;

pub(crate) struct SkeletonPirateSpawn;

impl SpawnAction for SkeletonPirateSpawn {
    fn spawn(&self, commands: &mut Commands, translation: Vec3) {
        commands.add(SendEventCommand(SpawnSkeletonPirateEvent {
            location: Transform::from_translation(translation),
        }));
    }
}

pub(crate) struct DefaultSpawn(pub(crate) Arc<String>);

impl SpawnAction for DefaultSpawn {
    fn spawn(&self, commands: &mut Commands, translation: Vec3) {
        commands.add(SendEventCommand(SpawnActorEvent {
            name: self.0.clone(),
            args: DefaultSpawnArgs {
                transform: Transform::from_translation(translation),
            },
        }));
    }
}
