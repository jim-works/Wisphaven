use bevy::prelude::*;
use bevy::render::primitives::{Frustum, Sphere};

use crate::chunk::{ChunkCoord, CHUNK_SIZE_F32};
use crate::chunk_loading::ChunkLoader;
use crate::level::Level;
use crate::mesher::is_chunk_ready_for_meshing;
use util::LocalRepeatingTimer;

use super::NeedsMesh;

const CHUNK_ORDER_UPDATE_MS: u64 = 134; //don't want things to line up on 100ms all the time

pub fn set_meshing_order(
    mut chunk_query: Query<(&ChunkCoord, &mut NeedsMesh)>,
    loader_query: Query<(&GlobalTransform, &ChunkLoader)>,
    frustums: Query<&Frustum, With<Camera3d>>,
    level: Res<Level>,
    mut timer: Local<LocalRepeatingTimer<{ CHUNK_ORDER_UPDATE_MS }>>,
    time: Res<Time>,
) {
    timer.tick(time.delta());
    if !timer.just_finished() {
        return;
    }
    for (chunk_coord, mut needs_mesh) in chunk_query.iter_mut() {
        if !is_chunk_ready_for_meshing(*chunk_coord, &level) {
            //early exit if chunk can't be meshed
            needs_mesh.order = None;
            continue;
        }

        let chunk_pos = chunk_coord.to_vec3();
        let mut min_score = usize::MAX;
        //order is determined by distance to a meshing loader and presence in a view frustum
        for (tf, loader) in loader_query.iter() {
            //distance to meshing loader
            if !loader.mesh {
                continue; //don't care about this loader if it doesn't mesh
            }
            let score = (chunk_pos.distance(tf.translation()) / CHUNK_SIZE_F32) as usize;
            min_score = min_score.min(score);
        }

        //if loader is in a view frustum, is gets a discount on its order
        const FRUSTUM_ORDER_DENOM: usize = 10;
        for frust in frustums.iter() {
            if frust.intersects_sphere(
                &Sphere {
                    radius: CHUNK_SIZE_F32,
                    center: chunk_pos.into(),
                },
                false,
            ) {
                min_score /= FRUSTUM_ORDER_DENOM;
                break;
            }
        }

        if min_score == usize::MAX {
            needs_mesh.order = None;
        } else {
            needs_mesh.order = Some(min_score);
        }
    }
}
