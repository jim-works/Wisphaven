use bevy::prelude::*;

use crate::{
    physics::{
        collision::Aabb,
        movement::{GravityMult, Velocity},
        PhysicsBundle, PhysicsSystemSet,
    },
    ui::debug::FixedUpdateBlockGizmos,
    util::{plugin::SmoothLookTo, SendEventCommand},
    world::LevelLoadState,
    BlockCoord, BlockPhysics, Level,
};

use super::{ActorName, ActorResources, CombatInfo, CombatantBundle, Idler};

#[derive(Resource)]
pub struct GhostResources {
    pub scene: Handle<Scene>,
}

#[derive(Component, Default)]
pub struct Ghost;

#[derive(Component)]
pub struct Float {
    //relative to top of attached aabb
    pub target_ground_dist: f32,
    //relative to bottom of attached aabb
    pub target_ceiling_dist: f32,
    pub max_force: f32,
    pub ground_aabb_scale: Vec3, //scale to consider more blocks (so you start floating before coming into contact)
}

impl Default for Float {
    fn default() -> Self {
        Self {
            target_ground_dist: 2.5,
            target_ceiling_dist: 2.5,
            max_force: 0.04,
            ground_aabb_scale: Vec3::splat(1.5),
        }
    }
}

#[derive(Event)]
pub struct SpawnGhostEvent {
    pub location: GlobalTransform,
}

pub struct GhostPlugin;

impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(OnEnter(LevelLoadState::Loaded), trigger_spawning)
            .add_systems(Update, spawn_ghost)
            .add_systems(FixedUpdate, update_floater.in_set(PhysicsSystemSet::Main))
            .add_event::<SpawnGhostEvent>();
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(GhostResources {
        scene: assets.load("ghost/ghost.gltf#Scene0"),
    });
}

fn trigger_spawning(mut writer: EventWriter<SpawnGhostEvent>) {
    for i in 0..5 {
        writer.send(SpawnGhostEvent {
            location: GlobalTransform::from_xyz(
                (i % 5) as f32 * -5.0,
                (i / 5) as f32 * 5.0 + 50.0,
                0.0,
            ),
        });
    }
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("ghost"),
        Box::new(|commands, tf| commands.add(SendEventCommand(SpawnGhostEvent { location: tf }))),
    );
}

fn spawn_ghost(
    mut commands: Commands,
    res: Res<GhostResources>,
    mut spawn_requests: EventReader<SpawnGhostEvent>,
) {
    for spawn in spawn_requests.read() {
        commands.spawn((
            SceneBundle {
                scene: res.scene.clone_weak(),
                transform: spawn.location.compute_transform(),
                ..default()
            },
            Name::new("ghost"),
            CombatantBundle {
                combat_info: CombatInfo {
                    knockback_multiplier: 2.0,
                    ..CombatInfo::new(10.0, 0.0)
                },
                ..default()
            },
            PhysicsBundle {
                collider: Aabb::centered(Vec3::splat(0.5)),
                gravity: GravityMult(0.1),
                ..default()
            },
            Ghost,
            Idler::default(),
            SmoothLookTo::new(0.5),
            bevy::pbr::CubemapVisibleEntities::default(),
            bevy::render::primitives::CubemapFrusta::default(),
        ));
    }
}

fn get_float_delta_velocity(desired_height_change: f32, interpolate_speed: f32) -> f32 {
    //derivative at x=0 = interpolate speed
    //range scaled to (-speed, speed)
    let sigmoid = interpolate_speed * (-1.0 + 2.0 / (1.0 + (-2.0 * desired_height_change).exp()));
    sigmoid
}

fn update_floater(
    mut query: Query<(&mut Velocity, &mut Float, &Transform, &Aabb)>,
    physics_query: Query<&BlockPhysics>,
    level: Res<Level>,
    mut block_gizmos: ResMut<FixedUpdateBlockGizmos>,
) {
    const CHECK_MULT: f32 = 2.0;
    for (mut v, mut float, tf, aabb) in query.iter_mut() {
        //the ground has check area slightly larger than the actual hitbox to climb walls
        let ground_area = aabb.scale(float.ground_aabb_scale).move_min(Vec3::new(
            0.0,
            -float.target_ground_dist * CHECK_MULT,
            0.0,
        ));
        //the ceiling doesn't, because then we couldn't climb walls (would cancel out with the ground)
        let ceiling_area =
            aabb.add_size(Vec3::new(0.0, float.target_ceiling_dist * CHECK_MULT, 0.0));
        //should move this into a function, but difficult to make borrow checker happy
        let ground_overlaps =
            level.get_blocks_in_volume(ground_area.to_block_volume(tf.translation));
        let ground_blocks = ground_overlaps
            .iter()
            .filter_map(|(coord, block)| {
                block
                    .and_then(|b| b.entity())
                    .and_then(|e| physics_query.get(e).ok().and_then(|p| Aabb::from_block(p)))
                    .and_then(|b| Some((coord.to_vec3(), coord, b)))
            })
            .filter(move |(pos, _, b)| ground_area.intersects_aabb(tf.translation, *b, *pos));
        //now for ceiling blocks
        let ceiling_overlaps =
            level.get_blocks_in_volume(ceiling_area.to_block_volume(tf.translation));
        let ceiling_blocks = ceiling_overlaps
            .iter()
            .filter_map(|(coord, block)| {
                block
                    .and_then(|b| b.entity())
                    .and_then(|e| physics_query.get(e).ok().and_then(|p| Aabb::from_block(p)))
                    .and_then(|b| Some((coord.to_vec3(), b)))
            })
            .filter(move |(coord, b)| ceiling_area.intersects_aabb(tf.translation, *b, *coord));
        let collider_top = aabb.world_max(tf.translation).y;
        let collider_bot = aabb.world_min(tf.translation).y;
        let mut ground_y = None;
        let mut ceiling_y = None;
        //ceiling will be lowest point above the top of the floater's collider
        for (pos, block_col) in ceiling_blocks {
            let block_bot = block_col.world_min(pos).y;
            if collider_top <= block_bot {
                //possible ceiling
                ceiling_y = Some(if let Some(y) = ceiling_y {
                    block_bot.min(y)
                } else {
                    block_bot
                });
            }
        }
        //ground will be the highest point below the bottom of the floater's collider
        //ground has to have an exposed block above it
        for (pos, coord, block_col) in ground_blocks {
            let block_top = block_col.world_max(pos).y;
            if collider_bot >= block_top
                && ground_overlaps
                    .get(coord + BlockCoord::new(0, 1, 0))
                    .and_then(|t| t.entity())
                    .and_then(|e| physics_query.get(e).ok().map(|p| !p.is_solid()))
                    .unwrap_or(true)
            {
                //possible ground
                block_gizmos.blocks.insert(coord);
                ground_y = Some(if let Some(y) = ground_y {
                    block_top.max(y)
                } else {
                    block_top
                });
            }
        }

        let target_y = match (
            ground_y.map(|y| y + aabb.min().y + float.target_ground_dist),
            ceiling_y.map(|y| y - aabb.max().y - float.target_ceiling_dist),
        ) {
            (None, None) => {
                //not close enough to ground, cop out
                continue;
            }
            (None, Some(y)) => y,
            (Some(y), None) => y,
            (Some(ground_y), Some(ceiling_y)) => 0.5 * (ground_y + ceiling_y), //take avg if we are in the middle
        };

        let delta_v = get_float_delta_velocity(target_y - tf.translation.y, float.max_force);
        if delta_v < 0.0 && ceiling_y.is_none() || delta_v > 0.0 && ground_y.is_none() {
            //don't want to get pulled down to the ground or pushed up to the ceiling
            continue;
        }
        if v.0.y * delta_v.signum() > delta_v.abs() {
            //we are already moving in the right direction faster than the floater would push, so do nothing
            continue;
        }
        v.0.y += delta_v;
    }
}
