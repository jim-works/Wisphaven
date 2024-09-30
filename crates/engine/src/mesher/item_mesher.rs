use bevy::{pbr::ExtendedMaterial, prelude::*};

use util::{image::ImageExtension, palette::Palette};

use crate::{
    items::{inventory::Inventory, ItemIcon, ItemName},
    world::{
        chunk::{Chunk, ChunkCoord, FatChunkIdx, BLOCKS_PER_FAT_CHUNK},
        util::BlockPalette,
        BlockMesh, BlockMeshShape,
    },
};

use super::{
    extended_materials::{ColorArrayExtension, TextureArrayExtension},
    materials::chunk_base_material,
    mesh_chunk, ChunkMaterial, ChunkMesh, MeshData,
};

pub struct ItemMesherPlugin;

impl Plugin for ItemMesherPlugin {
    fn build(&self, app: &mut App) {
        //only want to clear these once the meshes are generated
        app.init_resource::<Events<GenerateItemMeshEvent>>()
            .add_systems(
                PreUpdate,
                generate_item_meshes.run_if(resource_exists::<HeldItemResources>),
            )
            .add_systems(
                Update,
                visualize_held_item.run_if(resource_exists::<HeldItemResources>),
            )
            .add_systems(
                Update,
                setup_held_item
                    .run_if(resource_exists::<ChunkMaterial>)
                    .run_if(not(resource_exists::<HeldItemResources>)),
            );
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
    pub material: ItemMeshMaterial,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ItemMeshMaterial {
    ColorArray,
    TextureArray,
}

#[derive(Resource)]
pub struct HeldItemResources {
    pub color_material: Handle<ExtendedMaterial<StandardMaterial, ColorArrayExtension>>,
    pub texture_material: Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>,
}
#[derive(Component)]
pub struct VisualizeHeldItem {
    pub inventory: Entity,
}

fn setup_held_item(
    mut color_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, ColorArrayExtension>>>,
    chunk: Res<ChunkMaterial>,
    mut commands: Commands,
) {
    if let Some(tex_mat) = &chunk.transparent_material {
        commands.insert_resource(HeldItemResources {
            color_material: color_materials.add(ExtendedMaterial {
                base: chunk_base_material(),
                extension: ColorArrayExtension {},
            }),
            texture_material: tex_mat.clone(),
        })
    }
}

fn visualize_held_item(
    mut commands: Commands,
    //ugly but idc
    mut color_held_query: Query<
        (Entity, &VisualizeHeldItem, &mut Handle<Mesh>),
        (
            With<Handle<ExtendedMaterial<StandardMaterial, ColorArrayExtension>>>,
            Without<Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>>,
        ),
    >,
    mut texture_held_query: Query<
        (Entity, &VisualizeHeldItem, &mut Handle<Mesh>),
        (
            With<Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>>,
            Without<Handle<ExtendedMaterial<StandardMaterial, ColorArrayExtension>>>,
        ),
    >,
    inv_query: Query<&Inventory>,
    item_query: Query<&ItemMesh>,
    res: Res<HeldItemResources>,
) {
    //color materials
    for (entity, held, mut mesh) in color_held_query.iter_mut() {
        if let Ok(inv) = inv_query.get(held.inventory) {
            if let Some(item) = inv.selected_item_entity() {
                if let Ok(item_mesh) = item_query.get(item) {
                    info!("set hand item mesh to color {:?}", item_mesh.mesh);
                    *mesh = item_mesh.mesh.clone();
                    if item_mesh.material == ItemMeshMaterial::TextureArray {
                        //we have color material attached, need to switch to texture
                        commands.entity(entity).remove::<Handle<ExtendedMaterial<StandardMaterial, ColorArrayExtension>>>()
                            .insert(res.texture_material.clone());
                    }
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

    //texture materials
    for (entity, held, mut mesh) in texture_held_query.iter_mut() {
        if let Ok(inv) = inv_query.get(held.inventory) {
            if let Some(item) = inv.selected_item_entity() {
                if let Ok(item_mesh) = item_query.get(item) {
                    *mesh = item_mesh.mesh.clone();
                    info!("set hand item mesh to texture {:?}", item_mesh.mesh);
                    if item_mesh.material == ItemMeshMaterial::ColorArray {
                        //we have texture material attached, need to switch to color
                        commands.entity(entity).remove::<Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>>()
                            .insert(res.color_material.clone());
                    }
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
                material: res.color_material.clone(),
                transform: tf,
                ..default()
            },
            VisualizeHeldItem { inventory },
        ))
        .id()
}

pub fn generate_item_meshes(
    mut events: ResMut<Events<GenerateItemMeshEvent>>,
    item_query: Query<(&ItemIcon, Option<&ItemName>)>,
    images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    const ALPHA_CUTOFF: f32 = 0.05;
    for GenerateItemMeshEvent(item) in events.drain() {
        info!("received generate item mesh event");
        //use the cnunk meshing code so we get AO, whatever for free
        //and less work
        if let Ok((item_icon_handle, opt_name)) = item_query.get(item) {
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
                                if color.alpha() <= ALPHA_CUTOFF {
                                    continue;
                                }
                                let layer = u32::from_le_bytes(color.to_srgba().to_u8_array());
                                fat_chunk.blocks.set(
                                    Into::<usize>::into(FatChunkIdx::new(
                                        0,
                                        (height - y) as i8,
                                        x as i8,
                                    )),
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
                for vert in chunk_mesh.transparent.verts.iter_mut() {
                    *vert = *vert / 16.0;
                }
                let item_mesh = chunk_mesh.transparent.create_mesh(&mut meshes);
                commands.entity(item).insert(ItemMesh {
                    mesh: item_mesh,
                    material: ItemMeshMaterial::ColorArray,
                });
                info!("created item mesh for {:?}", opt_name);
            } else {
                error!("tried to create item mesh when icon image isn't ready!");
            }
        } else {
            error!("tried to create item mesh when item doesn't have icon!")
        }
    }
}
