use bevy::{pbr::ExtendedMaterial, prelude::*};
use materials::{ColorArrayExtension, TextureArrayExtension};

#[derive(Resource)]
pub struct HeldItemResources {
    pub color_material: Handle<ExtendedMaterial<StandardMaterial, ColorArrayExtension>>,
    pub texture_material: Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>,
}

impl HeldItemResources {
    pub fn create_held_item_visualizer(
        &self,
        commands: &mut Commands,
        inventory: Entity,
        tf: Transform,
    ) -> Entity {
        #[allow(state_scoped_entities)]
        commands
            .spawn((
                MeshMaterial3d(self.color_material.clone()),
                Mesh3d::default(),
                tf,
                crate::components::VisualizeHeldItem { inventory },
            ))
            .id()
    }
}
