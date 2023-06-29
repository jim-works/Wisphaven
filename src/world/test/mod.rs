use crate::serialization::ChunkSaveFormat;

use super::{chunk::*, *};

#[derive(Resource)]
struct Counter(u32);

#[test]
fn test_buffer() {
    let mut app = App::new();
    app.insert_resource(Level::new("test", 1));
    app.insert_resource(Counter(0));
    app.add_systems((normal_system, rle_system, spawn_chunks).chain());
    app.update();

    let level = app.world.resource::<Level>();
    //test that buffers do anything
    assert_eq!(
        level.get_block(BlockCoord::new(10, 10, 10)),
        Some(BlockType::Basic(0))
    );
    assert_eq!(
        level.get_block(BlockCoord::new(-CHUNK_SIZE_I32 + 10, 10, 10)),
        Some(BlockType::Basic(1))
    );
    drop(level);

    app.update();

    let level = app.world.resource::<Level>();
    //test that buffers don't overwrite everything
    assert_eq!(
        level.get_block(BlockCoord::new(10, 10, 10)),
        Some(BlockType::Basic(0))
    );
    assert_eq!(
        level.get_block(BlockCoord::new(-CHUNK_SIZE_I32 + 10, 10, 10)),
        Some(BlockType::Basic(1))
    );
    assert_eq!(
        level.get_block(BlockCoord::new(11, 10, 10)),
        Some(BlockType::Basic(1))
    );
    assert_eq!(
        level.get_block(BlockCoord::new(-CHUNK_SIZE_I32 + 11, 10, 10)),
        Some(BlockType::Basic(2))
    );
}

fn normal_system(mut commands: Commands, level: Res<Level>, counter: Res<Counter>) {
    let mut normal_buffer = BlockBuffer::new();
    normal_buffer.set(
        BlockCoord::new(10 + counter.0 as i32, 10, 10),
        BlockChange::Set(BlockType::Basic(counter.0)),
    );
    level.add_buffer(normal_buffer, &mut commands)
}

fn rle_system(mut commands: Commands, level: Res<Level>, counter: Res<Counter>) {
    let mut chunk = Chunk::new(ChunkCoord::new(-1, 0, 0), commands.spawn_empty().id());
    chunk[ChunkIdx::new(10 + counter.0 as u8, 10, 10)] = BlockType::Basic(counter.0 + 1);
    level.add_rle_buffer(
        chunk.position,
        &ChunkSaveFormat::from(&chunk).data,
        &mut commands,
    )
}

fn spawn_chunks(mut commands: Commands, level: Res<Level>, mut counter: ResMut<Counter>) {
    if counter.0 == 0 {
        let chunk = Chunk::new(ChunkCoord::new(-1, 0, 0), commands.spawn_empty().id());
        level.add_chunk(chunk.position, ChunkType::Full(chunk));
        let chunk = Chunk::new(ChunkCoord::new(0, 0, 0), commands.spawn_empty().id());
        level.add_chunk(chunk.position, ChunkType::Full(chunk));
    }
    counter.0 += 1;
}
