pub mod mesher;

use bevy::prelude::*;

pub struct MesherPlugin;

impl Plugin for MesherPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(mesher::mesh_new);
    }
}