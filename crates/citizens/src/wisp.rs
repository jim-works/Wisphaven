use std::{array, f32::consts::PI};

use bevy::prelude::*;

use ai::attacker::{AggroClosestEnemy, UseItemAction};
use big_brain::prelude::*;
use interfaces::{
    resources::HeldItemResources,
    scheduling::{LevelLoadState, LevelSystemSet, PhysicsLevelSet},
};
use physics::{
    PhysicsBundle,
    collision::{Aabb, BlockPhysics},
    movement::{Gravity, GravityMult, Mass, Velocity},
};
use serde::Deserialize;
use util::{lerp, plugin::SmoothLookTo};

use engine::actors::{
    ActorName, ActorResources, BuildActorRegistry, Combatant, CombatantBundle, IdleAction, Idler,
    SpawnActorEvent,
    ai::scorers::AggroScorer,
    ghost::{Float, GhostResources, Handed, OrbitParticle},
    team::PLAYER_TEAM,
};
use world::{FixedUpdateBlockGizmos, level::Level};

const WISP_PARTICLE_COUNT: usize = 7;
#[derive(Resource)]
pub struct WispResources {
    pub center_mesh: Handle<Mesh>,
    pub particle_mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub particle_materials: [Handle<StandardMaterial>; WISP_PARTICLE_COUNT],
}

#[derive(Component, Default)]
pub struct Wisp;

#[derive(Event, Deserialize, Default, Debug, Clone)]
pub struct SpawnWisp {
    pub handed: Handed,
    #[serde(skip)] //todo - this will probably come from inventory or something later
    pub hat: Option<ClothingVisual>,
    #[serde(skip)]
    pub front: Option<ClothingVisual>,
}

#[derive(Debug, Clone)]
pub struct ClothingVisual {
    pub scene: Handle<Scene>,
    // there are default offsets for different armor slots, this should just be adjust the mesh to look good
    pub offset: Transform,
}

impl ClothingVisual {
    pub fn spawn(&self, cb: &mut ChildBuilder, default_offset: Vec3) {
        cb.spawn((
            SceneRoot(self.scene.clone()),
            self.offset
                .with_translation(self.offset.translation + default_offset),
        ));
    }
}

#[derive(Event, Debug, Clone)]
pub struct PopulateWisp(pub Entity, pub Name, pub SpawnActorEvent<SpawnWisp>);

pub(crate) struct WispPlugin;

impl Plugin for WispPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(
                FixedUpdate,
                (spawn_wisp, populate_wisp)
                    .chain()
                    .in_set(LevelSystemSet::Tick),
            )
            .add_systems(FixedUpdate, update_floater.in_set(PhysicsLevelSet::Main))
            .add_event::<PopulateWisp>()
            .add_actor::<SpawnWisp>(ActorName::core("wisp"));
    }
}

fn load_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const CENTER_PARTICLE_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);
    const OUTER_PARTICLE_COLOR: Color = Color::srgb(0.96, 0.90, 1.0);
    let particle_materials: [Handle<StandardMaterial>; WISP_PARTICLE_COUNT as usize] =
        array::from_fn(|n| {
            let progress = (n + 1) as f32 / (WISP_PARTICLE_COUNT + 1) as f32;
            materials.add(StandardMaterial {
                base_color: Color::mix(&CENTER_PARTICLE_COLOR, &OUTER_PARTICLE_COLOR, progress),
                ..default()
            })
        });
    commands.insert_resource(WispResources {
        center_mesh: meshes.add(Mesh::from(Cuboid::from_corners(
            Vec3::new(-0.3, -0.5, -0.3),
            Vec3::new(0.3, 0.5, 0.3),
        ))),
        particle_mesh: meshes.add(Mesh::from(Cuboid::from_length(1.0))),
        material: materials.add(StandardMaterial {
            base_color: CENTER_PARTICLE_COLOR,
            ..default()
        }),
        particle_materials,
    });
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry
        .add_dynamic::<SpawnWisp>(ActorName::core("wisp"));
}

fn spawn_wisp(
    mut commands: Commands,
    mut writer: EventWriter<PopulateWisp>,
    mut spawn_requests: EventReader<SpawnActorEvent<SpawnWisp>>,
    mut name: Local<Name>,
) {
    if name.is_empty() {
        *name = Name::new("wisp");
    }
    for spawn in spawn_requests.read() {
        writer.send(PopulateWisp(
            commands.spawn_empty().id(),
            name.clone(),
            spawn.clone(),
        ));
    }
}

pub(crate) fn populate_wisp(
    mut commands: Commands,
    res: Res<WispResources>,
    ghost_res: Res<GhostResources>,
    held_item_resources: Res<HeldItemResources>,
    mut populate_requests: EventReader<PopulateWisp>,
) {
    const MIN_PARTICLE_SIZE: f32 = 0.225;
    const MAX_PARTICLE_SIZE: f32 = 0.7;
    const MIN_PARTICLE_DIST: f32 = 0.15;
    const MAX_PARTICLE_DIST: f32 = 0.5;
    const MIN_PARTICLE_SPEED: f32 = 0.05;
    const MAX_PARTICLE_SPEED: f32 = 0.2;
    const ATTACK_RANGE: f32 = 25.0;
    for PopulateWisp(wisp_entity, name, spawn) in populate_requests.read() {
        let Some(mut ec) = commands.get_entity(*wisp_entity) else {
            error!(
                "cannot get entity commands for invalid wisp entity {:?}",
                wisp_entity
            );
            continue;
        };
        ec.insert_if_new((
            StateScoped(LevelLoadState::Loaded),
            MeshMaterial3d(res.material.clone()),
            Mesh3d(res.center_mesh.clone()),
            name.clone(),
            spawn
                .transform
                .with_translation(spawn.transform.translation + Vec3::Y),
            CombatantBundle {
                combatant: Combatant::new(10.0, 0.),
                team: PLAYER_TEAM,
                ..default()
            },
            PhysicsBundle {
                collider: Aabb::centered(Vec3::new(0.8, 1.0, 0.8)),
                mass: Mass(0.5),
                ..default()
            },
            Float::default(),
            Wisp,
            Idler::default(),
            SmoothLookTo::new(0.5),
            AggroClosestEnemy {
                range: ATTACK_RANGE,
                ..default()
            },
            Thinker::build()
                .label("wisp thinker")
                .picker(Highest)
                .when(FixedScore::build(0.01), IdleAction { seconds: 0.5 })
                .when(
                    AggroScorer {
                        range: ATTACK_RANGE,
                    },
                    UseItemAction { slot: 0 },
                ),
        ))
        .with_children(|children| {
            // spawn clothing if applicable
            if let Some(hat) = &spawn.event.hat {
                hat.spawn(children, Vec3::new(0., 0.5, 0.));
            }
            if let Some(front) = &spawn.event.front {
                front.spawn(children, Vec3::new(0., -1., -0.375))
            }
            //orbit particles
            for (i, point) in (0..WISP_PARTICLE_COUNT).zip(
                util::iterators::even_distribution_on_sphere(WISP_PARTICLE_COUNT as u32),
            ) {
                //size and distance are inversely correlated
                let size = lerp(
                    MAX_PARTICLE_SIZE,
                    MIN_PARTICLE_SIZE,
                    i as f32 / WISP_PARTICLE_COUNT as f32,
                );
                let dist = lerp(
                    MIN_PARTICLE_DIST,
                    MAX_PARTICLE_DIST,
                    i as f32 / WISP_PARTICLE_COUNT as f32,
                );
                let speed = lerp(
                    MIN_PARTICLE_SPEED,
                    MAX_PARTICLE_SPEED,
                    i as f32 / WISP_PARTICLE_COUNT as f32,
                );
                let material = res.particle_materials[i as usize].clone();
                let angle_inc = 2.0 * PI / WISP_PARTICLE_COUNT as f32;
                let angle = i as f32 * angle_inc;
                children.spawn((
                    MeshMaterial3d(material),
                    Mesh3d(res.particle_mesh.clone()),
                    Transform::from_translation(point * dist).with_scale(Vec3::splat(size)),
                    OrbitParticle::stable(
                        dist,
                        Vec3::new(speed * angle.sin(), 0.0, speed * angle.cos()),
                    ),
                ));
            }
        });
        //right hand
        let right_hand_entity = engine::actors::ghost::spawn_ghost_hand(
            *wisp_entity,
            spawn.transform,
            Vec3::new(0.5, -0.2, -0.6),
            Vec3::new(0.6, 0.2, -0.5),
            0.15,
            Quat::default(),
            &ghost_res,
            &mut commands,
        );
        //left hand
        let left_hand_entity = engine::actors::ghost::spawn_ghost_hand(
            *wisp_entity,
            spawn.transform,
            Vec3::new(-0.5, -0.2, -0.6),
            Vec3::new(-0.6, 0.2, -0.5),
            0.15,
            Quat::default(),
            &ghost_res,
            &mut commands,
        );
        spawn.event.handed.assign_hands(
            *wisp_entity,
            left_hand_entity,
            right_hand_entity,
            &mut commands,
        );
        let item_visualizer = held_item_resources.create_held_item_visualizer(
            &mut commands,
            *wisp_entity,
            Transform::from_scale(Vec3::splat(4.0)).with_translation(Vec3::new(0.0, -1.0, -3.4)),
        );
        commands
            .entity(right_hand_entity)
            .add_child(item_visualizer);
    }
}

fn get_float_delta_velocity(float: &mut Float, desired_height_change: f32, gravity: f32) -> f32 {
    // there will likely be some steady state error, but adding an I term makes the controller more unstable
    let derivative = desired_height_change - float.last_error;
    float.last_error = desired_height_change;
    let kp = 0.005;
    let kd = 0.02;
    return kp * desired_height_change + kd * derivative - gravity;
}

fn update_floater(
    mut query: Query<(
        &mut Velocity,
        &mut Float,
        &GlobalTransform,
        &Aabb,
        &GravityMult,
    )>,
    physics_query: Query<&BlockPhysics>,
    level: Res<Level>,
    gravity: Res<Gravity>,
    mut block_gizmos: ResMut<FixedUpdateBlockGizmos>,
) {
    const CHECK_MULT: f32 = 2.0;
    for (mut v, mut float, gtf, aabb, gravity_mult) in query.iter_mut() {
        let translation = gtf.translation();
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
        let ground_overlaps = level.get_blocks_in_volume(ground_area.to_block_volume(translation));
        let ground_blocks = ground_overlaps
            .iter()
            .filter_map(|(coord, block)| {
                block
                    .and_then(|b| b.entity())
                    .and_then(|e| physics_query.get(e).ok().and_then(Aabb::from_block))
                    .map(|b| (coord.as_vec3(), coord, b))
            })
            .filter(move |(pos, _, b)| ground_area.intersects_aabb(translation, *b, *pos));
        //now for ceiling blocks
        let ceiling_overlaps =
            level.get_blocks_in_volume(ceiling_area.to_block_volume(translation));
        let ceiling_blocks = ceiling_overlaps
            .iter()
            .filter_map(|(coord, block)| {
                block
                    .and_then(|b| b.entity())
                    .and_then(|e| physics_query.get(e).ok().and_then(Aabb::from_block))
                    .map(|b| (coord.as_vec3(), b))
            })
            .filter(move |(coord, b)| ceiling_area.intersects_aabb(translation, *b, *coord));
        let collider_top = aabb.world_max(translation).y;
        let collider_bot = aabb.world_min(translation).y;
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
                    .get(coord + IVec3::new(0, 1, 0))
                    .and_then(|t| t.entity())
                    .and_then(|e| physics_query.get(e).ok().map(|p| !p.is_solid()))
                    .unwrap_or(true)
            {
                //possible ground
                block_gizmos.blocks.insert(coord.into());
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

        let delta_v = get_float_delta_velocity(
            &mut float,
            target_y - translation.y,
            gravity.y * gravity_mult.0,
        );
        if delta_v < -gravity.y.abs() && ceiling_y.is_none()
            || gravity.y.abs() > 0.0 && ground_y.is_none()
        {
            //don't want to get pulled down to the ground or pushed up to the ceiling
            continue;
        }
        v.0.y += delta_v;
    }
}
