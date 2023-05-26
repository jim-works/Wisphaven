use bevy::prelude::*;
use dashmap::DashMap;
use super::{chunk::*, octree::{Octree, LeafOctreeNode, OctreeNodeData, OctreeCoord}};

#[derive(Resource)]
pub struct Level {
    pub chunks: DashMap<ChunkCoord, ChunkType, ahash::RandomState>,
    pub octree: Octree
}

impl Level {
    pub fn new() -> Level {
        Level {
            chunks: DashMap::with_hasher(ahash::RandomState::new()),
            octree: Octree::new()
        }
    }
    pub fn add_chunk(&mut self, key: ChunkCoord, chunk: ChunkType) {
        if let ChunkType::Full(c) = chunk {
            self.octree.insert(Box::new(LeafOctreeNode::new(OctreeNodeData::new(0,OctreeCoord{x:key.x,y:key.y,z:key.z}), c.entity)));
            self.chunks.insert(key,ChunkType::Full(c));
        } else {
            self.chunks.insert(key,chunk);
        }
           
    }
}