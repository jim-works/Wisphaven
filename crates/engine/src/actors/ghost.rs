use std::{array, f32::consts::PI};

use bevy::{math::primitives, prelude::*};
use serde::{Deserialize, Serialize};
use util::{ease_in_back, ease_in_out_quad, iterators::*, lerp, plugin::SmoothLookTo};

use world::{FixedUpdateBlockGizmos, level::Level, settings::GraphicsSettings};

use physics::{
    PhysicsBundle,
    collision::{Aabb, BlockPhysics},
    movement::{GravityMult, Mass, Velocity},
};

use interfaces::{
    components::{Hand, HandState, SwingHand, UseHand},
    resources::HeldItemResources,
    scheduling::*,
};

use crate::items::{
    HitResult, ItemName, ItemResources, ItemStack, StartSwingingItemEvent, StartUsingItemEvent,
    SwingEndEvent, UseEndEvent,
    inventory::Inventory,
    item_attributes::{ItemSwingSpeed, ItemUseSpeed},
};

use super::{
    ActorName, ActorResources, BuildActorRegistry, Combatant, CombatantBundle, Idler,
    SpawnActorEvent, abilities::stamina::Stamina,
};

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
    pub ground_aabb_scale: Vec3, //scale to consider more blocks (so you start floating before coming into contact)

    pub last_error: f32,
}

impl Default for Float {
    fn default() -> Self {
        Self {
            target_ground_dist: 3.5,
            target_ceiling_dist: 3.5,
            ground_aabb_scale: Vec3::splat(1.5),
            last_error: 0.0,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct FloatBoost {
    pub extra_height: f32,
    pub gravity_mult: f32,
    pub stamina_per_tick: f32,
    pub enabled: bool,
    effects_added: bool,
    delta_grav: f32,
}

impl Default for FloatBoost {
    fn default() -> Self {
        Self {
            extra_height: 3.0,
            gravity_mult: 0.0,
            stamina_per_tick: 1.0 / 64.0,
            enabled: false,
            effects_added: false,
            delta_grav: 0.0,
        }
    }
}

impl FloatBoost {
    pub fn with_extra_height(self, extra_height: f32) -> Self {
        Self {
            extra_height,
            ..self
        }
    }
    pub fn with_gravity_mult(self, gravity_mult: f32) -> Self {
        Self {
            gravity_mult,
            ..self
        }
    }
    pub fn with_stamina_per_tick(self, stamina_per_tick: f32) -> Self {
        Self {
            stamina_per_tick,
            ..self
        }
    }
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Default, Debug)]
pub enum Handed {
    #[default]
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
                    ec.try_insert((
                        SwingHand {
                            hand: left_hand,
                            miss_offset: Vec3::new(0.0, 0.0, 1.0), //idk default for now
                        },
                        UseHand {
                            hand: right_hand,
                            miss_offset: Vec3::new(0.0, 0.4, 0.0), //idk default for now
                        },
                    ));
                }
            }
            Handed::Right => {
                if let Some(mut ec) = commands.get_entity(entity) {
                    ec.try_insert((
                        SwingHand {
                            hand: right_hand,
                            miss_offset: Vec3::new(0.0, 0.0, 1.0), //idk default for now
                        },
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

#[derive(Event, Deserialize, Default, Debug)]
pub struct SpawnGhost {
    pub handed: Handed,
}

pub struct GhostPlugin;

impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_resources, add_to_registry))
            .add_systems(
                Update,
                (
                    spawn_ghost.in_set(LevelSystemSet::Main),
                    move_cube_orbit_particles,
                    update_ghost_hand,
                    ((windup_swing_hand, windup_use_hand), (swing_hand, use_hand)).chain(), //chain so that conflicts will resolve to the main animation
                ),
            )
            .add_systems(
                FixedUpdate,
                (float_boost).chain().in_set(PhysicsLevelSet::Main),
            )
            .add_actor::<SpawnGhost>(ActorName::core("ghost"));
    }
}

pub fn load_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const CENTER_PARTICLE_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);
    const OUTER_PARTICLE_COLOR: Color = Color::srgb(0.96, 0.90, 1.0);
    let particle_materials: [Handle<StandardMaterial>; GHOST_PARTICLE_COUNT as usize] =
        array::from_fn(|n| {
            let progress = (n + 1) as f32 / (GHOST_PARTICLE_COUNT + 1) as f32;
            materials.add(StandardMaterial {
                base_color: Color::mix(&CENTER_PARTICLE_COLOR, &OUTER_PARTICLE_COLOR, progress),
                ..default()
            })
        });
    commands.insert_resource(GhostResources {
        center_mesh: meshes.add(Mesh::from(primitives::Cuboid::from_corners(
            Vec3::new(-0.3, -0.5, -0.3),
            Vec3::new(0.3, 0.5, 0.3),
        ))),
        particle_mesh: meshes.add(Mesh::from(primitives::Cuboid::from_length(1.0))),
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

fn add_to_registry(mut res: ResMut<ActorResources>) {
    res.registry
        .add_dynamic::<SpawnGhost>(ActorName::core("ghost"));
}

fn spawn_ghost(
    mut commands: Commands,
    res: Res<GhostResources>,
    items: Res<ItemResources>,
    held_item_resources: Res<HeldItemResources>,
    mut spawn_requests: EventReader<SpawnActorEvent<SpawnGhost>>,
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
                StateScoped(LevelLoadState::Loaded),
                MeshMaterial3d(res.material.clone()),
                Mesh3d(res.center_mesh.clone()),
                spawn.transform,
                Name::new("ghost"),
                CombatantBundle {
                    combatant: Combatant::new(10.0, 0.),
                    ..default()
                },
                PhysicsBundle {
                    collider: Aabb::centered(Vec3::new(0.8, 1.0, 0.8)),
                    mass: Mass(0.5),
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
                    let material = res.particle_materials[i as usize].clone();
                    let angle_inc = 2.0 * PI / GHOST_PARTICLE_COUNT as f32;
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
            })
            .id();
        let mut inventory = Inventory::new(ghost_entity, 5);
        inventory.set_slot_no_events(
            0,
            ItemStack::new(
                items
                    .registry
                    .get_basic(&ItemName::core("ruby_pickaxe"))
                    .unwrap(),
                1,
            ),
        );
        commands.entity(ghost_entity).insert(inventory);
        //right hand
        let right_hand_entity = spawn_ghost_hand(
            ghost_entity,
            spawn.transform,
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
            spawn.transform,
            Vec3::new(-0.5, -0.2, -0.6),
            Vec3::new(-0.6, 0.2, -0.5),
            0.15,
            Quat::default(),
            &res,
            &mut commands,
        );
        spawn.event.handed.assign_hands(
            ghost_entity,
            left_hand_entity,
            right_hand_entity,
            &mut commands,
        );
        let item_visualizer = held_item_resources.create_held_item_visualizer(
            &mut commands,
            ghost_entity,
            Transform::from_scale(Vec3::splat(4.0)).with_translation(Vec3::new(0.0, -1.0, -3.4)),
        );
        commands
            .entity(right_hand_entity)
            .add_child(item_visualizer);
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
            StateScoped(LevelLoadState::Loaded),
            Transform::from_translation(owner_pos.transform_point(offset))
                .with_scale(Vec3::splat(hand_size)),
            Mesh3d(res.particle_mesh.clone()),
            MeshMaterial3d(res.hand_particle_material.clone()),
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
                    MeshMaterial3d(material),
                    Mesh3d(res.particle_mesh.clone()),
                    Transform::from_translation(point * dist).with_scale(Vec3::splat(size)),
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
    mut hand_query: Query<(&GlobalTransform, &mut Hand)>,
    mut swing_event: EventReader<StartSwingingItemEvent>,
) {
    for event in swing_event.read() {
        if let Ok(swing_hand) = owner_query.get(event.user) {
            if let Ok(swing_speed) = item_query.get(event.stack.id) {
                if let Ok((tf, mut hand)) = hand_query.get_mut(swing_hand.hand) {
                    //play windup animation for the items windup duration, if 0 duration, don't play anim
                    //simple xP
                    hand.state = HandState::Windup {
                        start_pos: tf.translation(),
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
    mut hand_query: Query<(&GlobalTransform, &mut Hand)>,
    mut swing_event: EventReader<StartUsingItemEvent>,
) {
    for event in swing_event.read() {
        if let Ok(use_hand) = owner_query.get(event.user) {
            if let Ok(use_speed) = item_query.get(event.stack.id) {
                if let Ok((tf, mut hand)) = hand_query.get_mut(use_hand.hand) {
                    //play windup animation for the items windup duration, if 0 duration, don't play anim
                    //simple xP
                    hand.state = HandState::Windup {
                        start_pos: tf.translation(),
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
    mut hand_query: Query<(&GlobalTransform, &mut Hand)>,
    mut swing_event: EventReader<SwingEndEvent>,
    settings: Res<GraphicsSettings>,
) {
    for event in swing_event.read() {
        if let Ok(swing_hand) = owner_query.get(event.user) {
            if let Ok(swing_speed) = item_query.get(event.stack.id) {
                if let Ok((tf, mut hand)) = hand_query.get_mut(swing_hand.hand) {
                    //we already waiting for the windup, but we need a small amount of time for the animation to play
                    //subtract that time off the backswing if we have the budget so the animation still has the correct total duration
                    //if we don't have the time budget, it will be slightly off. better than no animation though!
                    hand.state = HandState::Hitting {
                        start_pos: tf.translation(),
                        target: match event.result {
                            HitResult::Hit(pos) => pos,
                            HitResult::Miss | HitResult::Fail => {
                                tf.transform_point(swing_hand.miss_offset)
                            }
                        },
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
    mut hand_query: Query<(&GlobalTransform, &mut Hand)>,
    mut use_event: EventReader<UseEndEvent>,
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
                        start_pos: tf.translation(),
                        target: match event.result {
                            HitResult::Hit(p) => p,
                            HitResult::Miss | HitResult::Fail => {
                                tf.transform_point(use_hand.miss_offset)
                            }
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
    mut query: Query<(Entity, &mut Transform, &mut Hand, Option<&Parent>)>,
    ghost_query: Query<&Transform, Without<Hand>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut tf, mut hand, parent_opt) in query.iter_mut() {
        let owner = hand.owner;
        let offset = hand.offset;
        let windup_offset = hand.windup_offset;
        let Ok(ghost_tf) = ghost_query.get(owner) else {
            if let Some(ec) = commands.get_entity(entity) {
                ec.despawn_recursive();
            }
            continue;
        };

        //hand is a child of owner when following
        if let HandState::Following = hand.state {
            if parent_opt.is_none() {
                let Some(mut ec) = commands.get_entity(owner) else {
                    //owner despawned
                    commands.entity(entity).despawn_recursive();
                    continue;
                };
                ec.add_child(entity);
                tf.translation = hand.offset;
                tf.rotation = hand.rotation;
                continue;
            }
        } else if parent_opt.is_some() {
            commands.entity(entity).remove_parent();
            tf.translation = ghost_tf.transform_point(hand.offset);
            tf.rotation = ghost_tf.rotation * hand.rotation;
            continue;
        }

        match &mut hand.state {
            HandState::Following => {}
            HandState::Windup {
                start_pos,
                windup_time,
                time_remaining,
            } => {
                let dest = ghost_tf.transform_point(windup_offset);
                if *windup_time <= 0.0 {
                    tf.translation = dest;
                } else {
                    tf.translation = start_pos.lerp(
                        dest,
                        ease_in_out_quad((*windup_time - *time_remaining) / (*windup_time)),
                    );
                    let t = *time_remaining - time.delta_secs();
                    *time_remaining = t.max(0.0);
                }

                //hold at top of windup until hit event comes through, then transition to hitting
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
                    *hit_time_remaining -= time.delta_secs();
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
                let target = ghost_tf.transform_point(offset);
                if *return_time <= 0.0 {
                    tf.translation = target;
                    hand.state = HandState::Following;
                } else {
                    tf.translation = start_pos.lerp(
                        target,
                        ease_in_out_quad((*return_time - *return_time_remaining) / (*return_time)),
                    );
                    *return_time_remaining -= time.delta_secs();
                    if *return_time_remaining <= 0.0 {
                        hand.state = HandState::Following;
                    }
                }
            }
        }
    }
}

fn move_cube_orbit_particles(
    mut query: Query<(&mut Transform, &mut OrbitParticle)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (mut tf, mut particle) in query.iter_mut() {
        let delta = (particle.origin - tf.translation).normalize_or_zero();
        let g = particle.gravity;
        particle.vel += dt * delta * g;
        tf.translation += dt * particle.vel;
    }
}

fn float_boost(
    mut query: Query<(
        &mut Float,
        &mut FloatBoost,
        &mut GravityMult,
        Option<&mut Stamina>,
    )>,
) {
    for (mut float, mut boost, mut grav, stamina_opt) in query.iter_mut() {
        if boost.enabled {
            if let Some(mut stamina) = stamina_opt {
                stamina.change(-boost.stamina_per_tick);
                if stamina.current <= 0.0 {
                    boost.enabled = false;
                }
            }
        }
        //I wish I could do this with Added<> queries and RemovedComponents<>, but if you remove a component in fixed update the added<> field doesn't get populated
        //I would have to keep a FloatBoostActive(bool) component where the bool decides if it's FOR REAL active
        //why do added and removed have to be treated differently? WHY NO ADDEDCOMPONENTS<>? WHY?
        if boost.enabled && !boost.effects_added {
            //add effects
            float.target_ground_dist += boost.extra_height;
            let new_grav = grav.with_scale(boost.gravity_mult);
            boost.delta_grav = grav.0 - new_grav.0;
            *grav = new_grav;
            boost.effects_added = true;
        } else if !boost.enabled && boost.effects_added {
            //clear effects
            float.target_ground_dist -= boost.extra_height;
            grav.0 += boost.delta_grav;
            boost.effects_added = false;
        }
    }
}
