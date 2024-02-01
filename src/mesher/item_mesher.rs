use bevy::prelude::*;

use crate::{
    chunk::{Chunk, ChunkCoord, FatChunkIdx, BLOCKS_PER_FAT_CHUNK},
    items::ItemIcon,
    util::{
        image::ImageExtension,
        palette::{BlockPalette, Palette},
    },
    BlockMesh, BlockMeshShape,
};

use super::{mesh_chunk, ChunkMesh, MeshData};

pub struct ItemMesherPlugin;

impl Plugin for ItemMesherPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GenerateItemMeshEvent>()
            .add_systems(Update, generate_item_meshes);
    }
}

#[derive(Event)]
pub struct GenerateItemMeshEvent(Entity);

pub fn generate_item_meshes(
    mut item_meshes: EventReader<GenerateItemMeshEvent>,
    item_query: Query<&ItemIcon>,
    images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
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
                                fat_chunk.blocks.set(
                                    Into::<usize>::into(FatChunkIdx::new(x as i8, 0, y as i8)),
                                    BlockMesh {
                                        use_transparent_shader: true,
                                        shape: BlockMeshShape::Uniform(0),
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
            }
        }
    }
}
