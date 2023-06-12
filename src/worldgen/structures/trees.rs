use bracket_noise::prelude::{FastNoise, NoiseType};

use crate::world::{chunk::*, BlockBuffer, BlockCoord, BlockType};

use super::StructureGenerator;

pub struct SmallTreeGenerator {
    rng: FastNoise,
    min_tree_height: f32,
    height_range: f32,
    leaf_size: i32,
    leaf_dist_from_top: i32
}
impl StructureGenerator for SmallTreeGenerator {
    fn rarity(&self) -> f32 {
        1.0
    }

    fn generate(
        &self,
        buffer: &mut BlockBuffer,
        pos: BlockCoord,
        local_pos: ChunkIdx,
        chunk: &Chunk,
    ) {
        //determine if location is suitable for a tree
        if !matches!(chunk[local_pos], BlockType::Basic(0)) {
            return;
        }
        for y in (local_pos.y + 1)..CHUNK_SIZE_U8 {
            if !matches!(chunk[ChunkIdx::new(local_pos.x, y, local_pos.z)], BlockType::Empty) {
                return;
            }
        }
        //place tree
        let height = (self.min_tree_height+self.height_range*self.rng.get_noise3d(pos.x as f32, pos.y as f32, pos.z as f32)) as i32;
        for y in 0..height {
            buffer.set_block(pos+BlockCoord::new(0,y,0), BlockType::Basic(4));
        }
        for x in -self.leaf_size..self.leaf_size+1 {
            for y in -self.leaf_size..self.leaf_size+1 {
                for z in -self.leaf_size..self.leaf_size+1 {
                    buffer.set_if_empty(pos+BlockCoord::new(x,y-self.leaf_dist_from_top+height,z), BlockType::Basic(5));
                }
            }
        }
    }
}
impl SmallTreeGenerator {
    pub fn new(seed: u64) -> Self {
        let mut noise = FastNoise::seeded(seed);
        noise.set_noise_type(NoiseType::WhiteNoise);
        SmallTreeGenerator {
            rng: noise,
            min_tree_height: 10.0,
            height_range: 10.0,
            leaf_size: 3,
            leaf_dist_from_top: 3
        }
    }
}
