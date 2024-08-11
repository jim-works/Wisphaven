use std::f32::consts::PI;

use bevy::prelude::*;

use crate::{util::{max_component_norm, DEG_TO_RAD, RAD_TO_DEG}, world::{chunk::{ChunkCoord, FatChunkIdx, BLOCKS_PER_CHUNK, CHUNK_SIZE, CHUNK_SIZE_I8}, BlockType}};


use super::{Direction, palette::BlockPalette};

#[cfg(test)]
mod iterators;
#[cfg(test)]
mod string;

#[test]
fn test_max_component_norm() {
    //test in each direction
    assert_eq!(Vec3::new(1.0,0.0,0.0), max_component_norm(Vec3::new(0.7,-0.5,0.5)));
    assert_eq!(Vec3::new(-1.0,0.0,0.0), max_component_norm(Vec3::new(-0.7,0.5,0.5)));
    assert_eq!(Vec3::new(0.0,1.0,0.0), max_component_norm(Vec3::new(0.5,0.7,0.5)));
    assert_eq!(Vec3::new(0.0,-1.0,0.0), max_component_norm(Vec3::new(-0.5,-0.7,0.5)));
    assert_eq!(Vec3::new(0.0,0.0,1.0), max_component_norm(Vec3::new(0.5,-0.5,0.7)));
    assert_eq!(Vec3::new(0.0,0.0,-1.0), max_component_norm(Vec3::new(-0.5,0.5,-0.7)));
}

#[test]
fn vec3_to_direction() {
    //test in each direction
    assert_eq!(Direction::PosX, Direction::from(Vec3::new(0.7,-0.5,0.5)));
    assert_eq!(Direction::NegX, Direction::from(Vec3::new(-0.7,0.5,0.5)));
    assert_eq!(Direction::PosY, Direction::from(Vec3::new(0.5,0.7,0.5)));
    assert_eq!(Direction::NegY, Direction::from(Vec3::new(-0.5,-0.7,0.5)));
    assert_eq!(Direction::PosZ, Direction::from(Vec3::new(0.5,-0.5,0.7)));
    assert_eq!(Direction::NegZ, Direction::from(Vec3::new(-0.5,0.5,-0.7)));
}

#[test]
fn test_create_fat_palette() {
    let mut app = App::new();

    app.add_systems(Update, |query: Query<&ChunkCoord>| {
        //make face, edges, and corner non-zero on a specific coordinate so we can easily verify that they were set properly
        let face_neighbor_entities: [ChunkCoord; 6] = core::array::from_fn(|i| ChunkCoord::new((i+1) as i32,0,0));
        let edge_neighbor_entities: [ChunkCoord; 12] = core::array::from_fn(|i| ChunkCoord::new(0,(i+1) as i32,0));
        let corner_neighbors: [Option<ChunkCoord>; 8] = core::array::from_fn(|i| Some(ChunkCoord::new(0,0,(i+1) as i32)));
        //main body will use default value (0 in all coordinates)
        let chunk: BlockPalette<BlockType, {BLOCKS_PER_CHUNK}> = BlockPalette::new(BlockType::Empty);
        let face_neighbors = core::array::from_fn(|i| Some([face_neighbor_entities[i]; BLOCKS_PER_CHUNK]));
        let edge_neighbors = core::array::from_fn(|i| Some([edge_neighbor_entities[i]; 16]));
        let palette = chunk.create_fat_palette(&query, face_neighbors, edge_neighbors, corner_neighbors);

        //corners
        assert!(palette[FatChunkIdx::new(-1,-1,-1).into()].z != 0, "was {:?}", palette[FatChunkIdx::new(-1,-1,-1).into()]);
        assert!(palette[FatChunkIdx::new(-1,-1,CHUNK_SIZE_I8).into()].z != 0, "was {:?}", palette[FatChunkIdx::new(-1,-1,CHUNK_SIZE_I8).into()]);
        assert!(palette[FatChunkIdx::new(-1,CHUNK_SIZE_I8,-1).into()].z != 0, "was {:?}", palette[FatChunkIdx::new(-1,CHUNK_SIZE_I8,-1).into()]);
        assert!(palette[FatChunkIdx::new(-1,CHUNK_SIZE_I8,CHUNK_SIZE_I8).into()].z != 0, "was {:?}", palette[FatChunkIdx::new(-1,CHUNK_SIZE_I8,CHUNK_SIZE_I8).into()]);
        assert!(palette[FatChunkIdx::new(CHUNK_SIZE_I8,-1,-1).into()].z != 0, "was {:?}", palette[FatChunkIdx::new(CHUNK_SIZE_I8,-1,-1).into()]);
        assert!(palette[FatChunkIdx::new(CHUNK_SIZE_I8,-1,CHUNK_SIZE_I8).into()].z != 0, "was {:?}", palette[FatChunkIdx::new(CHUNK_SIZE_I8,-1,CHUNK_SIZE_I8).into()]);
        assert!(palette[FatChunkIdx::new(CHUNK_SIZE_I8,CHUNK_SIZE_I8,-1).into()].z != 0, "was {:?}", palette[FatChunkIdx::new(CHUNK_SIZE_I8,CHUNK_SIZE_I8,-1).into()]);
        assert!(palette[FatChunkIdx::new(CHUNK_SIZE_I8,CHUNK_SIZE_I8,CHUNK_SIZE_I8).into()].z != 0, "was {:?}", palette[FatChunkIdx::new(CHUNK_SIZE_I8,CHUNK_SIZE_I8,CHUNK_SIZE_I8).into()]);

        //main chunk
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    assert_eq!(ChunkCoord::default(), palette[FatChunkIdx::new(x as i8,y as i8,z as i8).into()], "index ({}, {}, {})", x, y, z);
                }
            }
        }
    });

    app.update();

}

#[test]
fn test_deg_to_rad() {
    assert!((90.0*DEG_TO_RAD - PI/2.0).abs() < 0.00001);
    assert!((180.0*DEG_TO_RAD - PI).abs() < 0.00001);
}

#[test]
fn test_rad_to_deg() {
    assert!((90.0 - RAD_TO_DEG*PI/2.0).abs() < 0.00001);
    assert!((180.0 - RAD_TO_DEG*PI).abs() < 0.00001);
}