pub mod camera;
pub mod mesh_particles;
pub mod particles;

use bevy::prelude::*;

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraEffectsPlugin,
            particles::ParticlesPlugin,
            mesh_particles::MeshParticlesPlugin,
        ));
    }
}

pub const EFFECT_GRAVITY: Vec3 = Vec3 {
    x: 0.,
    y: -10.,
    z: 0.,
};
