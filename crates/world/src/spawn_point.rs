use bevy::prelude::*;
use util::iterators::Volume;

use crate::{level::Level, BlockType};

pub(crate) struct SpawnPointPlugin;

impl Plugin for SpawnPointPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnPoint>();
    }
}

#[derive(Resource, Default)]
pub struct SpawnPoint {
    pub base_point: Vec3,
}

impl SpawnPoint {
    pub fn get_spawn_point(&self, level: &Level) -> Vec3 {
        let mut calculated_spawn_point: IVec3 = self.base_point.as_ivec3();
        //checks for a 3x3x3 area of air above a 3x1x3 volume that contains at least one non-air block
        // we will check `CHECK_UP_RANGE` spawns iterating in the +Y direction, then return to where we started and `CHECK_DOWN_RANGE` spawns in the -Y direction
        // this repeats from each end `MAX_CHECKS` times.
        //the idea is to prefer spawning above ground, but prefer spawning in the shallow cave than some place way up in the sky.
        const MIN_SPAWN_VOLUME: IVec3 = IVec3::splat(3);
        const MAX_CHECKS: i32 = 100;
        const CHECK_UP_RANGE: i32 = 200;
        const CHECK_DOWN_RANGE: i32 = 10;
        for dy in (0..MAX_CHECKS)
            .flat_map(|i| (0..i * CHECK_UP_RANGE).chain((-i * CHECK_DOWN_RANGE..0).rev()))
        {
            calculated_spawn_point.y = dy;
            let ground_volume = Volume::new_inclusive(
                calculated_spawn_point + IVec3::NEG_Y,
                calculated_spawn_point + IVec3::new(MIN_SPAWN_VOLUME.x, -1, MIN_SPAWN_VOLUME.z),
            );
            let found_ground = ground_volume.iter().any(|coord| {
                // don't want to go super far if level isn't loaded yet
                matches!(
                    level.get_block(coord.into()),
                    Some(BlockType::Filled(_)) | None
                )
            });
            if !found_ground {
                continue;
            }
            let air_volume = Volume::new_inclusive(
                calculated_spawn_point + IVec3::Y,
                calculated_spawn_point + MIN_SPAWN_VOLUME,
            );
            let enough_air = air_volume.iter().all(|coord| {
                matches!(level.get_block(coord.into()), Some(BlockType::Empty) | None)
            });
            if !enough_air {
                continue;
            }
            info!("found spawn at {:?}", calculated_spawn_point);
            break;
        }
        let spawn_point = calculated_spawn_point.as_vec3() + MIN_SPAWN_VOLUME.as_vec3() / 2.;
        info!("spawn point is {:?}", spawn_point);
        spawn_point
    }
}
