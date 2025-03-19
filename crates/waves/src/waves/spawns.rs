use std::sync::Arc;

use bevy::prelude::*;

use actors::spawning::SpawnNamedActorEvent;
use engine::actors::{ActorName, SpawnActorEvent};
use util::SendEventCommand;

use super::SpawnAction;

pub struct DefaultSpawn(pub Arc<ActorName>);

impl SpawnAction for DefaultSpawn {
    fn spawn(&self, commands: &mut Commands, translation: Vec3) {
        commands.queue(SendEventCommand(SpawnNamedActorEvent {
            name: self.0.clone(),
            spawn_args: SpawnActorEvent::<()> {
                transform: Transform::from_translation(translation),
                event: (),
            },
            json_args: None,
        }));
    }
}

pub struct DirectSpawn<T>(pub T);

impl<T: Clone + Send + Sync + 'static> SpawnAction for DirectSpawn<T> {
    fn spawn(&self, commands: &mut Commands, translation: Vec3) {
        commands.queue(SendEventCommand(SpawnActorEvent::<T> {
            transform: Transform::from_translation(translation),
            event: self.0.clone(),
        }));
    }
}
