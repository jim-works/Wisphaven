use bevy::prelude::*;

use crate::{
    physics::{
        collision::Aabb,
        movement::{GravityMult, Velocity},
        PhysicsBundle, PhysicsSystemSet,
    },
    util::{plugin::SmoothLookTo, SendEventCommand},
    world::LevelLoadState,
    world_utils::blockcast_checkers,
    BlockPhysics, Level,
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
    pub target_ground_dist: f32,
    pub target_ceiling_dist: f32,
    pub max_force: f32,
    pub aabb_scale: Vec3, //scale to consider more blocks (so you start floating before coming into contact)
    last_velocity_applied: f32,
}

impl Default for Float {
    fn default() -> Self {
        Self {
            target_ground_dist: 2.0,
            target_ceiling_dist: 4.0,
            max_force: 0.02,
            aabb_scale: Vec3::splat(1.5),
            last_velocity_applied: Default::default(),
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
) {
    const CHECK_MULT: f32 = 10.0;
    for (mut v, mut float, tf, aabb) in query.iter_mut() {
        let area_to_check = aabb
            .scale(float.aabb_scale)
            .expand(Vec3::new(0.0, -float.target_ground_dist * CHECK_MULT, 0.0))
            .expand(Vec3::new(0.0, float.target_ceiling_dist * CHECK_MULT, 0.0));
        let target_ground_y = level
            .blockcast(
                tf.translation,
                Vec3::new(0.0, -float.target_ground_dist * CHECK_MULT, 0.0),
                |opt_block| blockcast_checkers::solid(&physics_query, opt_block),
            )
            .map(|hit| hit.hit_pos.y + float.target_ground_dist);
        let target_ceiling_y = level
            .blockcast(
                tf.translation,
                Vec3::new(0.0, float.target_ceiling_dist * CHECK_MULT, 0.0),
                |opt_block| blockcast_checkers::solid(&physics_query, opt_block),
            )
            .map(|hit| hit.hit_pos.y - float.target_ceiling_dist);
        let target_y = match (target_ground_y, target_ceiling_y) {
            (None, None) => {
                //not close enough to ground, cop out
                float.last_velocity_applied = 0.0;
                continue;
            }
            (None, Some(target_y)) => target_y,
            (Some(target_y), None) => target_y,
            (Some(target_ground), Some(target_ceiling)) => 0.5 * (target_ground + target_ceiling), //take avg if we are in the middle
        };

        let delta_v = get_float_delta_velocity(target_y - tf.translation.y, float.max_force);
        // v.0.y -= float.last_velocity_applied;
        // if delta_v < 0.0 && target_ceiling_y.is_none() || delta_v > 0.0 && target_ground_y.is_none()
        // {
        //     //don't want to get pulled down to the ground or pushed up to the ceiling
        //     float.last_velocity_applied = 0.0;
        //     continue;
        // }
        if v.0.y * delta_v.signum() > delta_v.abs() {
            //we are already moving in the right direction faster than the floater would push, so do nothing
            float.last_velocity_applied = 0.0;
            continue;
        }
        v.0.y += delta_v;
        float.last_velocity_applied = delta_v;
        info!(
            "{:?}, {:?}, target ground {:?}, target ceil {:?}, target_value {:?}",
            delta_v, float.max_force, target_ground_y, target_ceiling_y, target_y
        );
    }
}
