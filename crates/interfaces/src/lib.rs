pub mod components;
pub mod resources;
pub mod scheduling;
pub mod serialization;

use bevy::prelude::*;

pub struct InterfacesPlugin;

impl Plugin for InterfacesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(scheduling::SchedulingPlugin);
    }
}
