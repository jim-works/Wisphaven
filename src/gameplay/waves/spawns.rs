use bevy::prelude::*;

use crate::{actors::skeleton_pirate::SpawnSkeletonPirateEvent, util::SendEventCommand};

use super::SpawnAction;

pub struct SkeletonPirateSpawn;

impl SpawnAction for SkeletonPirateSpawn {
    fn spawn(&self, commands: &mut Commands, translation: Vec3) {
        commands.add(SendEventCommand(SpawnSkeletonPirateEvent {
            location: Transform::from_translation(translation),
        }))
    }
}
