use std::{f32::consts::PI, ops::Range};

use super::StructureGenerator;
use crate::{
    util::{
        get_next_prng,
        l_system::{LSystem, TreeAlphabet},
        ToSeed, prng_sample_range,
    },
    world::{chunk::*, BlockBuffer, BlockChange, BlockCoord, BlockId, BlockName, BlockRegistry},
};
use bevy::prelude::*;

//uses an L-system (https://en.wikipedia.org/wiki/L-system) to generate a tree
pub struct LTreeGenerator<
    P: Fn(&TreeAlphabet, u64) -> Option<Vec<TreeAlphabet>>,
    I: Fn(BlockCoord) -> Vec<TreeAlphabet>,
    B: Fn(&mut BlockBuffer<BlockId>, Vec3, Vec3),
    L: Fn(&mut BlockBuffer<BlockId>, BlockCoord),
> {
    l_system: LSystem<TreeAlphabet, P>,
    iterations: u64,
    initial_sentence: I,
    block_placer: B,
    leaf_placer: L,
    spawnable_block: BlockId,
}
impl<
        P: Fn(&TreeAlphabet, u64) -> Option<Vec<TreeAlphabet>>,
        I: Fn(BlockCoord) -> Vec<TreeAlphabet>,
        B: Fn(&mut BlockBuffer<BlockId>, Vec3, Vec3),
        L: Fn(&mut BlockBuffer<BlockId>, BlockCoord),
    > StructureGenerator for LTreeGenerator<P, I, B, L>
{
    fn rarity(&self) -> f32 {
        1.0
    }

    fn generate(
        &self,
        buffer: &mut BlockBuffer<BlockId>,
        pos: BlockCoord,
        local_pos: ChunkIdx,
        chunk: &GeneratingChunk,
    ) -> bool {
        let _my_span = info_span!("tree_validate", name = "tree_validate").entered();
        //determine if location is suitable for a tree
        if chunk[local_pos] != self.spawnable_block {
            return false;
        }
        for y in (local_pos.y + 1)..CHUNK_SIZE_U8 {
            if !matches!(
                chunk[ChunkIdx::new(local_pos.x, y, local_pos.z)],
                BlockId(crate::world::Id::Empty)
            ) {
                return false;
            }
        }
        let _my_span = info_span!("tree_generate", name = "tree_generate").entered();
        //generate tree
        let seed = pos.to_seed();
        let tree = self
            .l_system
            .iterate(&(self.initial_sentence)(pos), self.iterations, seed);
        //place tree
        let mut heads = Vec::new();
        let mut curr_head = Transform::from_translation(pos.to_vec3());
        for instruction in tree {
            match instruction {
                TreeAlphabet::Move(v) => {
                    let old_pos = curr_head.translation;
                    curr_head.translation += curr_head.forward() * v;
                    (self.block_placer)(buffer, old_pos, curr_head.translation);
                }
                TreeAlphabet::Rotate(r) => curr_head.rotate(r),
                TreeAlphabet::StartBranch => {
                    heads.push(curr_head);
                }
                TreeAlphabet::EndBranch => {
                    if let Some(h) = heads.pop() {
                        curr_head = h;
                    } else {
                        warn!("Branch end mismatch in L-tree at blockcoord: {:?}", pos);
                    }
                }
                TreeAlphabet::Replace(_) => {
                    (self.leaf_placer)(buffer, curr_head.translation.into());
                }
            }
        }
        false
    }
}
impl<
        P: Fn(&TreeAlphabet, u64) -> Option<Vec<TreeAlphabet>>,
        I: Fn(BlockCoord) -> Vec<TreeAlphabet>,
        B: Fn(&mut BlockBuffer<BlockId>, Vec3, Vec3),
        L: Fn(&mut BlockBuffer<BlockId>, BlockCoord),
    > LTreeGenerator<P, I, B, L>
{
    pub fn new(
        system: LSystem<TreeAlphabet, P>,
        iterations: u64,
        initial_sentence: I,
        block_placer: B,
        leaf_placer: L,
        spawnable_block: BlockId,
    ) -> Self {
        LTreeGenerator {
            l_system: system,
            iterations,
            initial_sentence,
            block_placer,
            leaf_placer,
            spawnable_block,
        }
    }
}

pub fn get_short_tree(
    seed: u64,
    trunk_height: Range<u64>,
    initial_branch_size: Range<u64>,
    branch_factor: f32,
    registry: &BlockRegistry,
) -> Box<dyn StructureGenerator + Send + Sync> {
    let wood = registry.get_id(&BlockName::core("log"));
    let leaves = registry.get_id(&BlockName::core("leaves"));
    let grass = registry.get_id(&BlockName::core("grass"));

    Box::new(LTreeGenerator::new(
        LSystem::new(move |x, idx| {
            const OPTIONS: usize = 4;
            let get_moves = |idx: usize, x: f32| -> Vec<TreeAlphabet> {
                let forward = TreeAlphabet::Move(x * branch_factor);
                let replace = TreeAlphabet::Replace(x * branch_factor);
                let rotate1 =
                    TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, PI / 6.0, 0.0, PI / 6.0));
                let rotate2 =
                    TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, PI / 6.0, 0.0, -PI / 6.0));
                let rotate3 = TreeAlphabet::Rotate(Quat::from_euler(
                    EulerRot::XYZ,
                    -PI / 6.0,
                    0.0,
                    -PI / 6.0,
                ));
                let rotate4 =
                    TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, -PI / 6.0, 0.0, 0.0));
                let rotate5 =
                    TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, -PI / 6.0));
                match idx.min(OPTIONS - 1) {
                    0 => vec![
                        TreeAlphabet::Move(x),
                        rotate1,
                        TreeAlphabet::StartBranch,
                        forward,
                        rotate4,
                        replace,
                        TreeAlphabet::EndBranch,
                        rotate3,
                        TreeAlphabet::StartBranch,
                        replace,
                        rotate2,
                        forward,
                        replace,
                        TreeAlphabet::EndBranch,
                        replace,
                    ],
                    1 => vec![
                        rotate5,
                        replace,
                        TreeAlphabet::StartBranch,
                        forward,
                        rotate4,
                        replace,
                        TreeAlphabet::EndBranch,
                        replace,
                    ],
                    2 => vec![
                        rotate1,
                        forward,
                        replace,
                        TreeAlphabet::StartBranch,
                        rotate5,
                        replace,
                        TreeAlphabet::EndBranch,
                        replace,
                    ],
                    3 => vec![
                        forward,
                        TreeAlphabet::StartBranch,
                        rotate2,
                        replace,
                        TreeAlphabet::EndBranch,
                        rotate3,
                        replace,
                        TreeAlphabet::StartBranch,
                        rotate4,
                        replace,
                        TreeAlphabet::EndBranch,
                    ],
                    _ => unreachable!(),
                }
            };
            match x {
                TreeAlphabet::Replace(x) => {
                    Some(get_moves(get_next_prng(idx.wrapping_add(seed)) as usize % OPTIONS, *x))
                }
                _ => None,
            }
        }),
        3,
        move |coord| {
            let seed = coord.to_seed().wrapping_add(seed);
            let branch_seed = get_next_prng(seed);
            vec![
                TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, PI * 0.5, 0.0, 0.0)),
                TreeAlphabet::Move(prng_sample_range(trunk_height.clone(), seed) as f32),
                TreeAlphabet::Replace(prng_sample_range(initial_branch_size.clone(), branch_seed) as f32),
            ]
        },
        move |p, a, b| p.place_descending(BlockChange::Set(wood), a.into(), b.into()),
        move |buffer, pos| {
            const LEAF_SIZE: i32 = 2;
            for x in -LEAF_SIZE..LEAF_SIZE + 1 {
                for y in -LEAF_SIZE..LEAF_SIZE + 1 {
                    for z in -LEAF_SIZE..LEAF_SIZE + 1 {
                        if x * x + y * y + z * z < LEAF_SIZE * LEAF_SIZE + 1 {
                            buffer.set(
                                BlockCoord::new(x, y, z) + pos,
                                BlockChange::SetIfEmpty(leaves),
                            );
                        }
                    }
                }
            }
        },
        grass,
    ))
}

pub fn get_cactus(
    seed: u64,
    first_height: Range<u64>,
    branch_factor: f32,
    iterations: u64,
    flower_denom: u64,
    registry: &BlockRegistry,
) -> Box<dyn StructureGenerator + Send + Sync> {
    let cactus = registry.get_id(&BlockName::core("cactus"));
    let cactus_flower = registry.get_id(&BlockName::core("cactus_flower"));
    let sand = registry.get_id(&BlockName::core("sand"));
    Box::new(LTreeGenerator::new(
        LSystem::new(move |x, idx| {
            //use random sample to select which production to use
            const OPTIONS: usize = 4;
            let get_moves = |idx: usize, x: f32| {
                let forward = TreeAlphabet::Move(x);
                let replace = TreeAlphabet::Replace(x * branch_factor);
                let rotate1 =
                    TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, PI / 2.0, 0.0, 0.0));
                let rotate2 =
                    TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, -PI / 2.0, 0.0, 0.0));
                let rotate3 =
                    TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, PI / 2.0));
                let rotate4 =
                    TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, -PI / 2.0));
                match idx.min(OPTIONS - 1) {
                    0 => vec![
                        TreeAlphabet::StartBranch,
                        rotate1,
                        forward,
                        replace,
                        TreeAlphabet::EndBranch,
                        TreeAlphabet::StartBranch,
                        rotate2,
                        forward,
                        replace,
                        TreeAlphabet::EndBranch,
                    ],
                    1 => vec![
                        TreeAlphabet::StartBranch,
                        rotate2,
                        forward,
                        replace,
                        TreeAlphabet::EndBranch,
                    ],
                    2 => vec![
                        TreeAlphabet::StartBranch,
                        rotate3,
                        forward,
                        replace,
                        TreeAlphabet::EndBranch,
                    ],
                    3 => vec![
                        TreeAlphabet::StartBranch,
                        rotate4,
                        forward,
                        replace,
                        TreeAlphabet::EndBranch,
                        TreeAlphabet::StartBranch,
                        rotate3,
                        forward,
                        replace,
                        TreeAlphabet::EndBranch,
                    ],
                    _ => unreachable!(),
                }
            };
            match x {
                TreeAlphabet::Replace(x) => {
                    Some(get_moves(get_next_prng(idx.wrapping_add(seed)) as usize % OPTIONS, *x))
                }
                _ => None,
            }
        }),
        iterations,
        move |coord| {
            let seed = coord.to_seed();
            let height = prng_sample_range(first_height.clone(), seed) as f32;
            vec![
                TreeAlphabet::Rotate(Quat::from_euler(EulerRot::XYZ, PI * 0.5, 0.0, 0.0)),
                TreeAlphabet::Move(height),
                TreeAlphabet::Replace(height*branch_factor),
                TreeAlphabet::Move(height*branch_factor+1.0) //want at least one block on top of the cactus
            ]
        },
        move |p, a, b| {
            p.place_descending_with_block(
                |pos| {
                    if get_next_prng(pos.to_seed().wrapping_add(seed)) % flower_denom == 0 {
                        BlockChange::SetIfEmpty(cactus_flower)
                    } else {
                        BlockChange::Set(cactus)
                    }
                },
                a.into(),
                b.into(),
            )
        },
        |_, _| {},
        sand,
    ))
}
