use bevy::{pbr::ExtendedMaterial, prelude::*};

use crate::{
    chunk::{Chunk, ChunkCoord, FatChunkIdx, BLOCKS_PER_FAT_CHUNK},
    items::{inventory::Inventory, ItemIcon},
    util::{
        image::ImageExtension,
        palette::{BlockPalette, Palette},
    },
    BlockMesh, BlockMeshShape,
};

use super::{
    extended_materials::ColorArrayExtension, materials::chunk_base_material, mesh_chunk, ChunkMesh,
    MeshData,
};

pub struct ItemMesherPlugin;

impl Plugin for ItemMesherPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GenerateItemMeshEvent>()
            .add_systems(PreUpdate, generate_item_meshes)
            .add_systems(Update, visualize_held_item)
            .add_systems(Startup, setup_held_item);
        app.add_plugins(
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, ColorArrayExtension>> {
                prepass_enabled: false,
                ..default()
            },
        );
    }
}

#[derive(Event)]
pub struct GenerateItemMeshEvent(pub Entity);

#[derive(Component, Clone)]
pub struct ItemMesh {
    pub mesh: Handle<Mesh>,
}

#[derive(Resource)]
pub struct HeldItemResources {
    pub material: Handle<ExtendedMaterial<StandardMaterial, ColorArrayExtension>>,
}
#[derive(Component)]
pub struct VisualizeHeldItem {
    pub inventory: Entity,
}

fn setup_held_item(
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, ColorArrayExtension>>>,
    mut commands: Commands,
) {
    commands.insert_resource(HeldItemResources {
        material: materials.add(ExtendedMaterial {
            base: chunk_base_material(),
            extension: ColorArrayExtension {},
        }),
    })
}

fn visualize_held_item(
    mut commands: Commands,
    mut held_query: Query<(Entity, &VisualizeHeldItem, &mut Handle<Mesh>)>,
    inv_query: Query<&Inventory>,
    item_query: Query<&ItemMesh>,
) {
    for (entity, held, mut mesh) in held_query.iter_mut() {
        if let Ok(inv) = inv_query.get(held.inventory) {
            if let Some(item) = inv.selected_item_entity() {
                if let Ok(item_mesh) = item_query.get(item) {
                    *mesh = item_mesh.mesh.clone();
                } else {
                    *mesh = Default::default();
                }
            } else {
                *mesh = Default::default();
            }
        } else {
            //owner despawned
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub fn create_held_item_visualizer(
    commands: &mut Commands,
    inventory: Entity,
    tf: Transform,
    res: &HeldItemResources,
) -> Entity {
    commands
        .spawn((
            MaterialMeshBundle::<ExtendedMaterial<StandardMaterial, ColorArrayExtension>> {
                material: res.material.clone(),
                transform: tf,
                ..default()
            },
            VisualizeHeldItem { inventory },
        ))
        .id()
}

pub fn generate_item_meshes(
    mut item_meshes: EventReader<GenerateItemMeshEvent>,
    item_query: Query<&ItemIcon>,
    images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    const ALPHA_CUTOFF: f32 = 0.05;
    for GenerateItemMeshEvent(item) in item_meshes.read() {
        //use the cnunk meshing code so we get AO, whatever for free
        //and less work
        if let Ok(item_icon_handle) = item_query.get(*item) {
            if let Some(item_icon) = images.get(&item_icon_handle.0) {
                let mut fat_chunk =
                    Chunk::<BlockPalette<BlockMesh, BLOCKS_PER_FAT_CHUNK>, BlockMesh> {
                        blocks: Box::new(BlockPalette::new(BlockMesh {
                            use_transparent_shader: false,
                            shape: BlockMeshShape::Empty,
                            single_mesh: None,
                        })),
                        entity: Entity::PLACEHOLDER,
                        position: ChunkCoord::new(0, 0, 0),
                        level: 1,
                        _data: std::marker::PhantomData,
                    };
                let mut chunk_mesh = ChunkMesh {
                    opaque: MeshData::default(),
                    transparent: MeshData::default(),
                    scale: 1.0,
                };
                let width = item_icon.texture_descriptor.size.width;
                let height = item_icon.texture_descriptor.size.height;
                for x in 0..width {
                    for y in 0..height {
                        match item_icon.get_color_at(x, y) {
                            Ok(color) => {
                                if color.a() <= ALPHA_CUTOFF {
                                    continue;
                                }
                                let layer = color.as_rgba_u32();
                                fat_chunk.blocks.set(
                                    Into::<usize>::into(FatChunkIdx::new(x as i8, 0, y as i8)),
                                    BlockMesh {
                                        use_transparent_shader: true,
                                        shape: BlockMeshShape::Uniform(layer),
                                        single_mesh: None,
                                    },
                                );
                            }
                            Err(e) => error!("Error creating item mesh: {:?}", e),
                        }
                    }
                }

                mesh_chunk(&fat_chunk, &mut chunk_mesh);
                let item_mesh = chunk_mesh.transparent.create_mesh(&mut meshes);
                commands.entity(*item).insert(ItemMesh { mesh: item_mesh });
            } else {
                error!("tried to create item mesh when icon image isn't ready!");
            }
        } else {
            error!("tried to create item mesh when item doesn't have icon!")
        }
    }
}
