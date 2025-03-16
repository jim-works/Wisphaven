use bevy::prelude::*;

use crate::{block::*, chunk::*, util::BlockPalette};
// use crate::serialization::ChunkSaveFormat;

// use super::{chunk::*, *};

// #[derive(Resource)]
// struct Counter(u32);

// #[test]
// fn test_buffer() {
//     let mut app = App::new();
//     app.insert_resource(Level::new("test", 1));
//     app.insert_resource(Counter(0));
//     app.add_systems((normal_system, rle_system, spawn_chunks).chain());
//     app.update();

//     let level = app.world.resource::<Level>();
//     //test that buffers do anything
//     assert_eq!(
//         level.get_block(BlockCoord::new(10, 10, 10)),
//         Some(BlockType::Basic(0))
//     );
//     assert_eq!(
//         level.get_block(BlockCoord::new(-CHUNK_SIZE_I32 + 10, 10, 10)),
//         Some(BlockType::Basic(1))
//     );
//     drop(level);

//     app.update();

//     let level = app.world.resource::<Level>();
//     //test that buffers don't overwrite everything
//     assert_eq!(
//         level.get_block(BlockCoord::new(10, 10, 10)),
//         Some(BlockType::Basic(0))
//     );
//     assert_eq!(
//         level.get_block(BlockCoord::new(-CHUNK_SIZE_I32 + 10, 10, 10)),
//         Some(BlockType::Basic(1))
//     );
//     assert_eq!(
//         level.get_block(BlockCoord::new(11, 10, 10)),
//         Some(BlockType::Basic(1))
//     );
//     assert_eq!(
//         level.get_block(BlockCoord::new(-CHUNK_SIZE_I32 + 11, 10, 10)),
//         Some(BlockType::Basic(2))
//     );
// }

// fn normal_system(mut commands: Commands, level: Res<Level>, counter: Res<Counter>) {
//     let mut normal_buffer = BlockBuffer::default();
//     normal_buffer.set(
//         BlockCoord::new(10 + counter.0 as i32, 10, 10),
//         BlockChange::Set(BlockType::Basic(counter.0)),
//     );
//     level.add_buffer(normal_buffer, &mut commands)
// }

// fn rle_system(mut commands: Commands, level: Res<Level>, counter: Res<Counter>) {
//     let mut chunk = Chunk::new(ChunkCoord::new(-1, 0, 0), commands.spawn_empty().id());
//     chunk[ChunkIdx::new(10 + counter.0 as u8, 10, 10)] = BlockType::Basic(counter.0 + 1);
//     level.add_rle_buffer(
//         chunk.position,
//         &ChunkSaveFormat::from(&chunk).data,
//         &mut commands,
//     )
// }

// fn spawn_chunks(mut commands: Commands, level: Res<Level>, mut counter: ResMut<Counter>) {
//     if counter.0 == 0 {
//         let chunk = Chunk::new(ChunkCoord::new(-1, 0, 0), commands.spawn_empty().id());
//         level.add_chunk(chunk.position, ChunkType::Full(chunk));
//         let chunk = Chunk::new(ChunkCoord::new(0, 0, 0), commands.spawn_empty().id());
//         level.add_chunk(chunk.position, ChunkType::Full(chunk));
//     }
//     counter.0 += 1;
// }

#[test]
fn test_create_fat_palette() {
    let mut app = App::new();

    app.add_systems(Update, |query: Query<&ChunkCoord>| {
        //make face, edges, and corner non-zero on a specific coordinate so we can easily verify that they were set properly
        let face_neighbor_entities: [ChunkCoord; 6] =
            core::array::from_fn(|i| ChunkCoord::new((i + 1) as i32, 0, 0));
        let edge_neighbor_entities: [ChunkCoord; 12] =
            core::array::from_fn(|i| ChunkCoord::new(0, (i + 1) as i32, 0));
        let corner_neighbors: [Option<ChunkCoord>; 8] =
            core::array::from_fn(|i| Some(ChunkCoord::new(0, 0, (i + 1) as i32)));
        //main body will use default value (0 in all coordinates)
        let chunk: BlockPalette<BlockType, { BLOCKS_PER_CHUNK }> =
            BlockPalette::new(BlockType::Empty);
        let face_neighbors =
            core::array::from_fn(|i| Some([face_neighbor_entities[i]; BLOCKS_PER_CHUNK]));
        let edge_neighbors = core::array::from_fn(|i| Some([edge_neighbor_entities[i]; 16]));
        let palette =
            chunk.create_fat_palette(&query, face_neighbors, edge_neighbors, corner_neighbors);

        //corners
        assert!(
            palette[FatChunkIdx::new(-1, -1, -1).into()].z != 0,
            "was {:?}",
            palette[FatChunkIdx::new(-1, -1, -1).into()]
        );
        assert!(
            palette[FatChunkIdx::new(-1, -1, CHUNK_SIZE_I8).into()].z != 0,
            "was {:?}",
            palette[FatChunkIdx::new(-1, -1, CHUNK_SIZE_I8).into()]
        );
        assert!(
            palette[FatChunkIdx::new(-1, CHUNK_SIZE_I8, -1).into()].z != 0,
            "was {:?}",
            palette[FatChunkIdx::new(-1, CHUNK_SIZE_I8, -1).into()]
        );
        assert!(
            palette[FatChunkIdx::new(-1, CHUNK_SIZE_I8, CHUNK_SIZE_I8).into()].z != 0,
            "was {:?}",
            palette[FatChunkIdx::new(-1, CHUNK_SIZE_I8, CHUNK_SIZE_I8).into()]
        );
        assert!(
            palette[FatChunkIdx::new(CHUNK_SIZE_I8, -1, -1).into()].z != 0,
            "was {:?}",
            palette[FatChunkIdx::new(CHUNK_SIZE_I8, -1, -1).into()]
        );
        assert!(
            palette[FatChunkIdx::new(CHUNK_SIZE_I8, -1, CHUNK_SIZE_I8).into()].z != 0,
            "was {:?}",
            palette[FatChunkIdx::new(CHUNK_SIZE_I8, -1, CHUNK_SIZE_I8).into()]
        );
        assert!(
            palette[FatChunkIdx::new(CHUNK_SIZE_I8, CHUNK_SIZE_I8, -1).into()].z != 0,
            "was {:?}",
            palette[FatChunkIdx::new(CHUNK_SIZE_I8, CHUNK_SIZE_I8, -1).into()]
        );
        assert!(
            palette[FatChunkIdx::new(CHUNK_SIZE_I8, CHUNK_SIZE_I8, CHUNK_SIZE_I8).into()].z != 0,
            "was {:?}",
            palette[FatChunkIdx::new(CHUNK_SIZE_I8, CHUNK_SIZE_I8, CHUNK_SIZE_I8).into()]
        );

        //main chunk
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    assert_eq!(
                        ChunkCoord::default(),
                        palette[FatChunkIdx::new(x as i8, y as i8, z as i8).into()],
                        "index ({}, {}, {})",
                        x,
                        y,
                        z
                    );
                }
            }
        }
    });

    app.update();
}
