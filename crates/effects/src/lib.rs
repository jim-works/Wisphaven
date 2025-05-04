use bevy::prelude::*;

pub mod mesh_particles;

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(mesh_particles::MeshParticlesPlugin);
    }
}
