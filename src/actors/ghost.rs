use std::{array, f32::consts::PI};

use bevy::prelude::*;

use crate::{
    items::{
        item_attributes::{ItemSwingSpeed, ItemUseSpeed},
        StartSwingingItemEvent, StartUsingItemEvent, SwingHitEvent, UseHitEvent,
    },
    physics::{collision::Aabb, movement::Velocity, PhysicsBundle, PhysicsSystemSet},
    settings::GraphicsSettings,
    ui::debug::FixedUpdateBlockGizmos,
    util::{
        ease_in_back, ease_in_out_quad, iterators::even_distribution_on_sphere, lerp,
        plugin::SmoothLookTo, SendEventCommand,
    },
    world::LevelLoadState,
    BlockCoord, BlockPhysics, Level,
};

use super::{ActorName, ActorResources, CombatInfo, CombatantBundle, Idler};

const GHOST_PARTICLE_COUNT: u32 = 7;
#[derive(Resource)]
pub struct GhostResources {
    pub center_mesh: Handle<Mesh>,
    pub particle_mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub particle_materials: [Handle<StandardMaterial>; GHOST_PARTICLE_COUNT as usize],
    pub hand_particle_material: Handle<StandardMaterial>,
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

#[derive(Component)]
pub struct Hand {
    pub owner: Entity,
    pub offset: Vec3,
    pub scale: f32,
    pub rotation: Quat,
    pub windup_offset: Vec3,
    pub state: HandState,
}

#[derive(Component)]
pub struct SwingHand(Entity);

#[derive(Component)]
pub struct UseHand {
    hand: Entity,
    //offset to play hit animation at if the item doesn't have a use coord (e.g. throwing a coin or not placing a block)
    miss_offset: Vec3,
}

#[derive(Debug)]
pub enum HandState {
    Following,
    Windup {
        start_pos: Vec3,
        windup_time: f32,
        time_remaining: f32,
    },
    Hitting {
        start_pos: Vec3,
        target: Vec3,
        hit_time: f32,
        return_time: f32,
        hit_time_remaining: f32,
    },
    Returning {
        start_pos: Vec3,
        return_time: f32,
        return_time_remaining: f32,
    },
}

#[derive(Component, Clone, Copy)]
pub enum Handed {
    Left,
    Right,
}

impl Handed {
    pub fn assign_hands(
        self,
        entity: Entity,
        left_hand: Entity,
        right_hand: Entity,
        commands: &mut Commands,
    ) {
        match self {
            Handed::Left => {
                if let Some(mut ec) = commands.get_entity(entity) {
                    ec.insert((
                        SwingHand(left_hand),
                        UseHand {
                            hand: right_hand,
                            miss_offset: Vec3::new(0.0, 0.4, 0.0), //idk default for now
                        },
                    ));
                }
            }
            Handed::Right => {
                if let Some(mut ec) = commands.get_entity(entity) {
                    ec.insert((
                        SwingHand(right_hand),
                        UseHand {
                            hand: left_hand,
                            miss_offset: Vec3::new(0.0, 0.4, 0.0), //idk default for now
                        },
                    ));
                }
            }
        };
    }
}

#[derive(Component, Copy, Clone, Default)]
pub struct OrbitParticle {
    pub gravity: f32,
    pub vel: Vec3,
    pub origin: Vec3, //local
    radius: f32,      //only for fun with stable
}

impl OrbitParticle {
    pub fn stable(radius: f32, vel: Vec3) -> Self {
        //a = v^2/r
        let v2 = vel.length_squared();
        Self {
            gravity: v2 / radius,
            vel,
            radius,
            ..default()
        }
    }
    pub fn update_stable_speed(&mut self, speed: f32) {
        let new_vel = self.vel.normalize_or_zero() * speed;
        self.gravity = speed * speed / self.radius;
        self.vel = new_vel;
    }
}

#[derive(Event)]
pub struct SpawnGhostEvent {
    pub location: Transform,
    pub handed: Handed,
}

pub struct GhostPlugin;

impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(OnEnter(LevelLoadState::Loaded), trigger_spawning)
            .add_systems(
                Update,
                (
                    spawn_ghost,
                    move_cube_orbit_particles,
                    update_ghost_hand,
                    ((windup_swing_hand, windup_use_hand), (swing_hand, use_hand)).chain(), //chain so that conflicts will resolve to the main animation
                ),
            )
            .add_systems(FixedUpdate, update_floater.in_set(PhysicsSystemSet::Main))
            .add_event::<SpawnGhostEvent>();
    }
}

pub fn load_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const CENTER_PARTICLE_COLOR: Color = Color::rgb(0.95, 0.95, 0.95);
    const OUTER_PARTICLE_COLOR: Color = Color::rgb(0.96, 0.90, 1.0);
    let particle_materials: [Handle<StandardMaterial>; GHOST_PARTICLE_COUNT as usize] =
        array::from_fn(|n| {
            let progress = (n + 1) as f32 / (GHOST_PARTICLE_COUNT + 1) as f32;
            materials.add(StandardMaterial {
                base_color: Color::hsl(
                    lerp(
                        CENTER_PARTICLE_COLOR.h(),
                        OUTER_PARTICLE_COLOR.h(),
                        progress,
                    ),
                    lerp(
                        CENTER_PARTICLE_COLOR.s(),
                        OUTER_PARTICLE_COLOR.s(),
                        progress,
                    ),
                    lerp(
                        CENTER_PARTICLE_COLOR.l(),
                        OUTER_PARTICLE_COLOR.l(),
                        progress,
                    ),
                ),
                ..default()
            })
        });
    commands.insert_resource(GhostResources {
        center_mesh: meshes.add(Mesh::from(shape::Box::from_corners(
            Vec3::new(-0.3, -0.5, -0.3),
            Vec3::new(0.3, 0.5, 0.3),
        ))),
        particle_mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(StandardMaterial {
            base_color: CENTER_PARTICLE_COLOR,
            ..default()
        }),
        particle_materials,
        hand_particle_material: materials.add(StandardMaterial {
            base_color: OUTER_PARTICLE_COLOR,
            ..default()
        }),
    });
}

fn trigger_spawning(mut writer: EventWriter<SpawnGhostEvent>) {
    for i in 0..5 {
        writer.send(SpawnGhostEvent {
            location: Transform::from_xyz((i % 5) as f32 * -5.0, (i / 5) as f32 * 5.0 + 50.0, 0.0)
                .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, PI, 0.0)),
            handed: if i % 2 == 0 {
                Handed::Right
            } else {
                Handed::Left
            },
        });
    }
}

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry.add_dynamic(
        ActorName::core("ghost"),
        Box::new(|commands, tf| {
            commands.add(SendEventCommand(SpawnGhostEvent {
                location: tf,
                handed: Handed::Right,
            }))
        }),
    );
}

fn spawn_ghost(
    mut commands: Commands,
    res: Res<GhostResources>,
    mut spawn_requests: EventReader<SpawnGhostEvent>,
) {
    const MIN_PARTICLE_SIZE: f32 = 0.225;
    const MAX_PARTICLE_SIZE: f32 = 0.7;
    const MIN_PARTICLE_DIST: f32 = 0.15;
    const MAX_PARTICLE_DIST: f32 = 0.5;
    const MIN_PARTICLE_SPEED: f32 = 0.05;
    const MAX_PARTICLE_SPEED: f32 = 0.2;
    for spawn in spawn_requests.read() {
        let ghost_entity = commands
            .spawn((
                PbrBundle {
                    material: res.material.clone(),
                    mesh: res.center_mesh.clone(),
                    transform: spawn.location,
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
                    collider: Aabb::centered(Vec3::new(0.8, 1.0, 0.8)),
                    ..default()
                },
                Float::default(),
                Ghost,
                Idler::default(),
                SmoothLookTo::new(0.5),
            ))
            .with_children(|children| {
                //orbit particles
                for (i, point) in
                    (0..GHOST_PARTICLE_COUNT).zip(even_distribution_on_sphere(GHOST_PARTICLE_COUNT))
                {
                    //size and distance are inversely correlated
                    let size = lerp(
                        MAX_PARTICLE_SIZE,
                        MIN_PARTICLE_SIZE,
                        i as f32 / GHOST_PARTICLE_COUNT as f32,
                    );
                    let dist = lerp(
                        MIN_PARTICLE_DIST,
                        MAX_PARTICLE_DIST,
                        i as f32 / GHOST_PARTICLE_COUNT as f32,
                    );
                    let speed = lerp(
                        MIN_PARTICLE_SPEED,
                        MAX_PARTICLE_SPEED,
                        i as f32 / GHOST_PARTICLE_COUNT as f32,
                    );
                    let material = (&res.particle_materials[i as usize]).clone();
                    let angle_inc = 2.0 * PI / GHOST_PARTICLE_COUNT as f32;
                    let angle = i as f32 * angle_inc;
                    children.spawn((
                        PbrBundle {
                            material,
                            mesh: res.particle_mesh.clone(),
                            transform: Transform::from_translation(point * dist)
                                .with_scale(Vec3::splat(size)),
                            ..default()
                        },
                        OrbitParticle::stable(
                            dist,
                            Vec3::new(speed * angle.sin(), 0.0, speed * angle.cos()),
                        ),
                    ));
                }
            })
            .id();
        //right hand
        let right_hand_entity = spawn_ghost_hand(
            ghost_entity,
            spawn.location,
            Vec3::new(0.5, -0.2, -0.6),
            Vec3::new(0.6, 0.2, -0.5),
            0.15,
            Quat::default(),
            &res,
            &mut commands,
        );
        //left hand
        let left_hand_entity = spawn_ghost_hand(
            ghost_entity,
            spawn.location,
            Vec3::new(-0.5, -0.2, -0.6),
            Vec3::new(-0.6, 0.2, -0.5),
            0.15,
            Quat::default(),
            &res,
            &mut commands,
        );
        spawn.handed.assign_hands(
            ghost_entity,
            left_hand_entity,
            right_hand_entity,
            &mut commands,
        )
    }
}

pub fn spawn_ghost_hand(
    owner: Entity,
    owner_pos: Transform,
    offset: Vec3,
    windup_offset: Vec3,
    hand_size: f32,
    hand_rot: Quat,
    res: &GhostResources,
    commands: &mut Commands,
) -> Entity {
    const HAND_PARTICLE_COUNT: u32 = 3;
    let min_particle_size: f32 = 0.1 / hand_size;
    let max_particle_size: f32 = 0.15 / hand_size;
    let min_particle_speed: f32 = 0.05 / hand_size;
    let max_particle_speed: f32 = 0.1 / hand_size;
    let min_particle_dist: f32 = 0.15 / hand_size;
    let max_particle_dist: f32 = 0.2 / hand_size;
    commands
        .spawn((
            PbrBundle {
                mesh: res.particle_mesh.clone(),
                material: res.hand_particle_material.clone(),
                transform: Transform::from_translation(owner_pos.transform_point(offset))
                    .with_scale(Vec3::splat(hand_size)),
                ..default()
            },
            Hand {
                owner,
                offset,
                windup_offset,
                scale: hand_size,
                rotation: hand_rot,
                state: HandState::Following,
            },
        ))
        .with_children(|children| {
            //orbit particles
            for (i, point) in
                (0..HAND_PARTICLE_COUNT).zip(even_distribution_on_sphere(HAND_PARTICLE_COUNT))
            {
                //size and distance are inversely correlated
                let size = lerp(
                    max_particle_size,
                    min_particle_size,
                    i as f32 / HAND_PARTICLE_COUNT as f32,
                );
                let dist = lerp(
                    min_particle_dist,
                    max_particle_dist,
                    i as f32 / HAND_PARTICLE_COUNT as f32,
                );
                let speed = lerp(
                    min_particle_speed,
                    max_particle_speed,
                    i as f32 / HAND_PARTICLE_COUNT as f32,
                );
                let material = res.hand_particle_material.clone();
                let angle_inc = 2.0 * PI / HAND_PARTICLE_COUNT as f32;
                let angle = i as f32 * angle_inc;
                children.spawn((
                    PbrBundle {
                        material,
                        mesh: res.particle_mesh.clone(),
                        transform: Transform::from_translation(point * dist)
                            .with_scale(Vec3::splat(size)),
                        ..default()
                    },
                    OrbitParticle::stable(
                        dist,
                        Vec3::new(speed * angle.sin(), 0.0, speed * angle.cos()),
                    ),
                ));
            }
        })
        .id()
}

fn windup_swing_hand(
    owner_query: Query<&SwingHand, Without<Hand>>,
    item_query: Query<&ItemSwingSpeed, Without<Hand>>,
    mut hand_query: Query<(&Transform, &mut Hand)>,
    mut swing_event: EventReader<StartSwingingItemEvent>,
) {
    for event in swing_event.read() {
        if let Ok(swing_hand) = owner_query.get(event.user) {
            if let Ok(swing_speed) = item_query.get(event.stack.id) {
                if let Ok((tf, mut hand)) = hand_query.get_mut(swing_hand.0) {
                    //play windup animation for the items windup duration, if 0 duration, don't play anim
                    //simple xP
                    hand.state = HandState::Windup {
                        start_pos: tf.translation,
                        windup_time: swing_speed.windup.as_secs_f32(),
                        time_remaining: swing_speed.windup.as_secs_f32(),
                    };
                }
            }
        }
    }
}

fn windup_use_hand(
    owner_query: Query<&UseHand, Without<Hand>>,
    item_query: Query<&ItemUseSpeed, Without<Hand>>,
    mut hand_query: Query<(&Transform, &mut Hand)>,
    mut swing_event: EventReader<StartUsingItemEvent>,
) {
    for event in swing_event.read() {
        if let Ok(use_hand) = owner_query.get(event.user) {
            if let Ok(use_speed) = item_query.get(event.stack.id) {
                if let Ok((tf, mut hand)) = hand_query.get_mut(use_hand.hand) {
                    //play windup animation for the items windup duration, if 0 duration, don't play anim
                    //simple xP
                    hand.state = HandState::Windup {
                        start_pos: tf.translation,
                        windup_time: use_speed.windup.as_secs_f32(),
                        time_remaining: use_speed.windup.as_secs_f32(),
                    };
                }
            }
        }
    }
}

fn swing_hand(
    owner_query: Query<&SwingHand, Without<Hand>>,
    item_query: Query<&ItemSwingSpeed, Without<Hand>>,
    mut hand_query: Query<(&Transform, &mut Hand)>,
    mut swing_event: EventReader<SwingHitEvent>,
    settings: Res<GraphicsSettings>,
) {
    for event in swing_event.read() {
        if let Ok(swing_hand) = owner_query.get(event.user) {
            if let Ok(swing_speed) = item_query.get(event.stack.id) {
                if let Ok((tf, mut hand)) = hand_query.get_mut(swing_hand.0) {
                    //we already waiting for the windup, but we need a small amount of time for the animation to play
                    //subtract that time off the backswing if we have the budget so the animation still has the correct total duration
                    //if we don't have the time budget, it will be slightly off. better than no animation though!
                    hand.state = HandState::Hitting {
                        start_pos: tf.translation,
                        target: event.pos,
                        hit_time: settings.hand_hit_animation_duration,
                        return_time: (swing_speed.backswing.as_secs_f32()
                            - settings.hand_hit_animation_duration)
                            .max(settings.hand_hit_animation_duration),
                        hit_time_remaining: settings.hand_hit_animation_duration,
                    };
                }
            }
        }
    }
}

fn use_hand(
    owner_query: Query<&UseHand, Without<Hand>>,
    item_query: Query<&ItemUseSpeed, Without<Hand>>,
    mut hand_query: Query<(&Transform, &mut Hand)>,
    mut use_event: EventReader<UseHitEvent>,
    settings: Res<GraphicsSettings>,
) {
    for event in use_event.read() {
        if let Ok(use_hand) = owner_query.get(event.user) {
            if let Ok(use_speed) = item_query.get(event.stack.id) {
                if let Ok((tf, mut hand)) = hand_query.get_mut(use_hand.hand) {
                    //we already waiting for the windup, but we need a small amount of time for the animation to play
                    //subtract that time off the backuse if we have the budget so the animation still has the correct total duration
                    //if we don't have the time budget, it will be slightly off. better than no animation though!
                    hand.state = HandState::Hitting {
                        start_pos: tf.translation,
                        target: match event.pos {
                            Some(p) => p,
                            None => tf.transform_point(use_hand.miss_offset),
                        },
                        hit_time: settings.hand_hit_animation_duration,
                        return_time: (use_speed.backswing.as_secs_f32()
                            - settings.hand_hit_animation_duration)
                            .max(settings.hand_hit_animation_duration),
                        hit_time_remaining: settings.hand_hit_animation_duration,
                    };
                }
            }
        }
    }
}

fn update_ghost_hand(
    mut query: Query<(Entity, &mut Transform, &mut Hand)>,
    ghost_query: Query<&Transform, Without<Hand>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut tf, mut hand) in query.iter_mut() {
        let owner = hand.owner;
        let offset = hand.offset;
        let rot = hand.rotation;
        let scale = hand.scale;
        let windup_offset = hand.windup_offset;
        match &mut hand.state {
            HandState::Following => {
                if let Ok(ghost_tf) = ghost_query.get(owner) {
                    tf.translation = ghost_tf.transform_point(offset);
                    tf.rotation = ghost_tf.rotation*rot;
                    tf.scale = ghost_tf.scale*scale;
                } else {
                    //invalid owner (most likely despawned)
                    commands.entity(entity).despawn_recursive();
                }
            }
            HandState::Windup {
                start_pos,
                windup_time,
                time_remaining,
            } => {
                if let Ok(ghost_tf) = ghost_query.get(owner) {
                    let dest = ghost_tf.transform_point(windup_offset);
                    if *windup_time <= 0.0 {
                        tf.translation = dest;
                    } else {
                        tf.translation = start_pos.lerp(
                            dest,
                            ease_in_out_quad((*windup_time - *time_remaining) / (*windup_time)),
                        );
                        let t = *time_remaining - time.delta_seconds();
                        *time_remaining = t.max(0.0);
                    }

                    //hold at top of windup until hit event comes through, then transition to hitting
                } else {
                    //invalid owner (most likely despawned)
                    commands.entity(entity).despawn_recursive();
                }
            }
            HandState::Hitting {
                start_pos,
                target,
                hit_time,
                hit_time_remaining,
                return_time,
            } => {
                if *hit_time <= 0.0 {
                    tf.translation = *target;
                    hand.state = HandState::Returning {
                        start_pos: tf.translation,
                        return_time: *return_time,
                        return_time_remaining: *return_time,
                    };
                } else {
                    tf.translation = start_pos.lerp(
                        *target,
                        ease_in_back((*hit_time - *hit_time_remaining) / (*hit_time)),
                    );
                    *hit_time_remaining -= time.delta_seconds();
                    if *hit_time_remaining <= 0.0 {
                        tf.translation = *target;
                        hand.state = HandState::Returning {
                            start_pos: tf.translation,
                            return_time: *return_time,
                            return_time_remaining: *return_time,
                        };
                    }
                }
            }
            HandState::Returning {
                return_time_remaining,
                start_pos,
                return_time,
            } => {
                if let Ok(ghost_tf) = ghost_query.get(owner) {
                    let target = ghost_tf.transform_point(offset);
                    if *return_time <= 0.0 {
                        tf.translation = target;
                        hand.state = HandState::Following;
                    } else {
                        tf.translation = start_pos.lerp(
                            target,
                            ease_in_out_quad(
                                (*return_time - *return_time_remaining) / (*return_time),
                            ),
                        );
                        *return_time_remaining -= time.delta_seconds();
                        if *return_time_remaining <= 0.0 {
                            hand.state = HandState::Following;
                        }
                    }
                } else {
                    //invalid owner (most likely despawned)
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }
}

fn move_cube_orbit_particles(
    mut query: Query<(&mut Transform, &mut OrbitParticle)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    for (mut tf, mut particle) in query.iter_mut() {
        let delta = (particle.origin - tf.translation).normalize_or_zero();
        let g = particle.gravity;
        particle.vel += dt * delta * g;
        tf.translation += dt * particle.vel;
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
    for (mut v, float, tf, aabb) in query.iter_mut() {
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
        if v.0.y * delta_v.signum() >= delta_v.abs() {
            //we are already moving in the right direction faster than the floater would push
            //slow down a bit to reduce bobbing
            let extra_v = v.0.y - delta_v;
            if extra_v.abs() > delta_v.abs() {
                v.0.y -= delta_v;
            } else {
                v.0.y -= extra_v;
            }
            continue;
        }
        v.0.y += delta_v;
    }
}
