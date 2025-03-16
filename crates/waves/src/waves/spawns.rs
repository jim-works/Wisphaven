use std::sync::Arc;

use bevy::prelude::*;

use actors::skeleton_pirate::SpawnSkeletonPirateEvent;
use actors::spawning::{DefaultSpawnArgs, SpawnActorEvent};
use util::SendEventCommand;

use super::SpawnAction;

pub struct SkeletonPirateSpawn;

impl SpawnAction for SkeletonPirateSpawn {
    fn spawn(&self, commands: &mut Commands, translation: Vec3) {
        commands.queue(SendEventCommand(SpawnSkeletonPirateEvent {
            location: Transform::from_translation(translation),
        }));
    }
}

pub struct DefaultSpawn(pub Arc<String>);

impl SpawnAction for DefaultSpawn {
    fn spawn(&self, commands: &mut Commands, translation: Vec3) {
        commands.queue(SendEventCommand(SpawnActorEvent {
            name: self.0.clone(),
            args: DefaultSpawnArgs {
                transform: Transform::from_translation(translation),
            },
        }));
    }
}
