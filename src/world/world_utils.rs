pub mod blockcast_checkers {
    use crate::{BlockPhysics, BlockType};
    use bevy::prelude::*;

    pub fn non_empty(opt_block: Option<BlockType>) -> bool {
        opt_block
            .map(|b| !matches!(b, BlockType::Empty))
            .unwrap_or(false)
    }

    pub fn empty(opt_block: Option<BlockType>) -> bool {
        opt_block
            .map(|b| matches!(b, BlockType::Empty))
            .unwrap_or(true)
    }

    pub fn solid(physics_query: &Query<&BlockPhysics>, opt_block: Option<BlockType>) -> bool {
        opt_block
            .map(|b| match b {
                BlockType::Empty => false,
                BlockType::Filled(e) => physics_query
                    .get(e)
                    .unwrap_or(&BlockPhysics::Empty)
                    .is_solid(),
            })
            .unwrap_or(false)
    }
}
