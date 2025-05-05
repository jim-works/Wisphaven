use crate::{
    actors::*,
    items::{
        HitResult, SpawnDroppedItemEvent, SwingEndEvent, UseEndEvent,
        inventory::{Inventory, ItemAction, ItemTargetPosition},
        item_attributes::{ItemSwingSpeed, ItemUseSpeed},
    },
};
use abilities::{
    dash::{CurrentlyDashing, Dash},
    stamina::Stamina,
};
use bevy::{prelude::*, window::CursorGrabMode};
use ghost::FloatBoost;
use interfaces::{components::Interactable, events::InteractedEvent, scheduling::*};
use leafwing_input_manager::prelude::ActionState;
use physics::{
    collision::{Aabb, BlockPhysics},
    grapple::Grappled,
    movement::{GravityMult, Velocity},
    query::{self, Raycast, RaycastHit},
};
use world::{
    events::{BlockHitEvent, BlockUsedEvent},
    level::Level,
    settings::Settings,
};

use super::{Action, MovementMode, TickMovement};

pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                rotate_mouse,
                boost_float_player,
                move_player,
                dash_player,
                follow_local_player,
                player_punch,
                player_use,
                toggle_player_flight,
                player_drop_item,
            )
                .in_set(LevelSystemSet::Main),
        )
        .add_systems(Update, update_window_focused)
        .insert_resource(CursorLocked(false));
    }
}

#[derive(Component, Default)]
pub struct RotateWithMouse {
    pub pitch: f32,
    pub roll: f32,
    pub yaw: f32,
    pub pitch_bound: f32,
    pub lock_pitch: bool,
    pub lock_yaw: bool,
}

#[derive(Component)]
pub struct FollowPlayer {
    pub offset: Vec3,
}

#[derive(Resource)]
pub struct CursorLocked(pub bool);

fn update_window_focused(mut focused: ResMut<CursorLocked>, query: Query<&Window>) {
    focused.0 = query
        .get_single()
        .map(|w| w.cursor_options.grab_mode != CursorGrabMode::None)
        .unwrap_or(false);
}

pub fn toggle_player_flight(
    mut query: Query<
        (&mut MovementMode, &mut GravityMult, &ActionState<Action>),
        With<ControlledPlayer>,
    >,
) {
    for (mut mode, mut gravity, action) in query.iter_mut() {
        if action.just_pressed(&Action::ToggleFlight) {
            match *mode {
                MovementMode::Flying => {
                    *mode = MovementMode::Normal;
                    gravity.set(1.0);
                    info!("Not Flying");
                }
                _ => {
                    *mode = MovementMode::Flying;
                    gravity.set(0.0);
                    info!("Flying");
                }
            };
        }
    }
}

pub fn move_player(
    mut query: Query<
        (
            &Transform,
            &mut TickMovement,
            &MovementMode,
            &ActionState<Action>,
        ),
        With<ControlledPlayer>,
    >,
) {
    // info!("Matches: {}", query.iter().len());
    for (tf, mut fm, mode, action) in query.iter_mut() {
        let mut dv = Vec3::ZERO;
        dv.z -= if action.pressed(&Action::MoveForward) {
            1.0
        } else {
            0.0
        };
        dv.z += if action.pressed(&Action::MoveBack) {
            1.0
        } else {
            0.0
        };
        dv.x += if action.pressed(&Action::MoveRight) {
            1.0
        } else {
            0.0
        };
        dv.x -= if action.pressed(&Action::MoveLeft) {
            1.0
        } else {
            0.0
        };

        let (y_rot, _, _) = tf.rotation.to_euler(EulerRot::YXZ);
        fm.0 = Quat::from_axis_angle(Vec3::Y, y_rot) * dv;

        if *mode == MovementMode::Flying {
            fm.0.y += if action.pressed(&Action::MoveUp) {
                1.0
            } else {
                0.0
            };
            fm.0.y -= if action.pressed(&Action::MoveDown) {
                1.0
            } else {
                0.0
            };
        }
    }
}

pub fn boost_float_player(
    mut query: Query<
        (Entity, &mut FloatBoost, &MovementMode, &ActionState<Action>),
        With<ControlledPlayer>,
    >,
    mut commands: Commands,
) {
    for (entity, mut fb, mode, action) in query.iter_mut() {
        fb.enabled = *mode != MovementMode::Flying && action.pressed(&Action::Float);
        if action.just_pressed(&Action::Float) {
            if let Some(mut ec) = commands.get_entity(entity) {
                ec.remove::<Grappled>();
            }
        }
    }
}

pub fn dash_player(
    mut query: Query<
        (Entity, &Dash, &mut Stamina, &Velocity, &ActionState<Action>),
        (With<ControlledPlayer>, Without<CurrentlyDashing>),
    >,
    mut commands: Commands,
    time: Res<Time>,
) {
    let current_time = time.elapsed();
    for (entity, dash, mut stamina, v, action) in query.iter_mut() {
        if action.just_pressed(&Action::Dash) && dash.stamina_cost.apply(&mut stamina) {
            if let Some(mut ec) = commands.get_entity(entity) {
                ec.try_insert(CurrentlyDashing::new(*dash, current_time, v.0));
            }
        }
    }
}

pub fn rotate_mouse(
    mut query: Query<(
        &mut Transform,
        &mut RotateWithMouse,
        &ActionState<Action>,
        Option<&LocalPlayer>,
    )>,
    focused: Res<CursorLocked>,
    settings: Res<Settings>,
) {
    // todo - figure out what to do with settings. ideally, we'd have clients have complete control over rotation
    let sensitivity = settings.mouse_sensitivity;
    for (mut tf, mut rotation, action, local) in query.iter_mut() {
        if local.is_some() && !focused.0 {
            // don't continue if we're in the inventory
            continue;
        }
        let delta = action.axis_pair(&Action::Look);
        if !rotation.lock_yaw {
            rotation.yaw -= delta.x * sensitivity;
        }
        if !rotation.lock_pitch {
            rotation.pitch -= delta.y * sensitivity;
        }

        rotation.pitch = rotation
            .pitch
            .clamp(-rotation.pitch_bound, rotation.pitch_bound);

        tf.rotation = Quat::from_axis_angle(Vec3::Y, rotation.yaw)
            * Quat::from_axis_angle(Vec3::X, rotation.pitch)
            * Quat::from_axis_angle(Vec3::Z, rotation.roll);
    }
}

pub fn follow_local_player(
    player_query: Query<(&Transform, &RotateWithMouse), With<LocalPlayer>>,
    mut follow_query: Query<
        (&FollowPlayer, &mut Transform, Option<&mut RotateWithMouse>),
        Without<LocalPlayer>,
    >,
) {
    if let Ok((player_tf, player_rot)) = player_query.get_single() {
        for (follow, mut follow_tf, opt_follow_rot) in follow_query.iter_mut() {
            follow_tf.translation = player_tf.translation + follow.offset;
            if let Some(mut follow_rot) = opt_follow_rot {
                follow_rot.yaw = player_rot.yaw;
            }
        }
    }
}

pub fn player_punch(
    mut player_query: Query<
        (
            Entity,
            &GlobalTransform,
            &Player,
            &Inventory,
            &ItemSwingSpeed,
            &mut ItemAction,
            &ActionState<Action>,
            Option<&LocalPlayer>,
        ),
        With<ControlledPlayer>,
    >,
    combat_query: Query<&Combatant>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    mut attack_punch_writer: EventWriter<AttackEvent>,
    mut block_hit_writer: EventWriter<BlockHitEvent>,
    mut swing_end_event: EventWriter<SwingEndEvent>,
    focused: Res<CursorLocked>,
    level: Res<Level>,
) {
    for (player_entity, tf, player, inv, swing_speed, mut item_action, action, local) in
        player_query.iter_mut()
    {
        if local.is_some() && !focused.0 {
            // don't continue if we're in the inventory
            continue;
        }
        if action.pressed(&Action::Punch) {
            let swang = item_action.try_swing(
                inv,
                crate::items::inventory::ItemTargetPosition::Entity(player_entity),
            );
            if swang && inv.selected_item().is_none() {
                // our default action will be punching
                match query::raycast(
                    Raycast::new(tf.translation(), tf.forward(), 10.0),
                    &level,
                    &block_physics_query,
                    &object_query,
                    &[player_entity],
                ) {
                    Some(RaycastHit::Block(hit_pos, hit_entity)) => {
                        block_hit_writer.send(BlockHitEvent {
                            item: None,
                            user: Some(player_entity),
                            block_position: hit_pos,
                            hit_forward: tf.forward(),
                        });
                        swing_end_event.send(SwingEndEvent {
                            user: player_entity,
                            slot: None,
                            speed: *swing_speed,
                            result: HitResult::Hit(hit_entity.hit_pos),
                        });
                    }
                    Some(RaycastHit::Object(hit)) => {
                        if combat_query.contains(hit.entity) {
                            attack_punch_writer.send(AttackEvent {
                                attacker: Some(player_entity),
                                target: hit.entity,
                                damage: player.hit_damage,
                                knockback: *tf.forward(),
                            });
                            swing_end_event.send(SwingEndEvent {
                                user: player_entity,
                                slot: None,
                                speed: *swing_speed,
                                result: HitResult::Hit(hit.hit_pos),
                            });
                        }
                    }
                    _ => {
                        swing_end_event.send(SwingEndEvent {
                            user: player_entity,
                            slot: None,
                            speed: *swing_speed,
                            result: HitResult::Miss,
                        });
                    }
                }
            }
        }
    }
}

pub fn player_use(
    mut player_query: Query<
        (
            Entity,
            &Inventory,
            &mut ItemAction,
            &ItemUseSpeed,
            &GlobalTransform,
            &ActionState<Action>,
            Option<&LocalPlayer>,
        ),
        With<ControlledPlayer>,
    >,
    focused: Res<CursorLocked>,
    level: Res<Level>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    usable_query: Query<&Interactable>,
    mut use_end_event: EventWriter<UseEndEvent>,
    mut block_use_writer: EventWriter<BlockUsedEvent>,
    mut commands: Commands,
) {
    for (entity, inv, mut item_action, use_speed, tf, action, local) in player_query.iter_mut() {
        if local.is_some() && !focused.0 {
            // don't continue if we're in the inventory
            continue;
        }
        if action.just_pressed(&Action::Use) && item_action.can_use() {
            //first test if we interacted with the world
            match query::raycast(
                Raycast::new(tf.translation(), tf.forward(), 10.0),
                &level,
                &block_physics_query,
                &object_query,
                &[entity],
            ) {
                Some(RaycastHit::Block(coord, hit_entity)) => {
                    if level.use_block(
                        coord,
                        entity,
                        tf.forward(),
                        &usable_query,
                        &mut block_use_writer,
                    ) && item_action.try_use_empty(ItemTargetPosition::Positon(
                        Transform::from_translation(hit_entity.hit_pos).into(),
                    )) {
                        //we used a block, so don't also use an item
                        use_end_event.send(UseEndEvent {
                            user: entity,
                            slot: None,
                            speed: *use_speed,
                            result: HitResult::Hit(hit_entity.hit_pos),
                        });
                        return;
                    }
                }
                Some(RaycastHit::Object(hit)) => {
                    if usable_query.contains(hit.entity)
                        && item_action.try_use_empty(ItemTargetPosition::Entity(hit.entity))
                    {
                        commands.trigger_targets(
                            InteractedEvent {
                                user: entity,
                                hit_pos: hit.hit_pos,
                            },
                            hit.entity,
                        );
                        //don't also use an item
                        use_end_event.send(UseEndEvent {
                            user: entity,
                            slot: None,
                            speed: *use_speed,
                            result: HitResult::Hit(hit.hit_pos),
                        });
                        return;
                    }
                }
                _ => (),
            }
            //we didn't use a block, so try to use an item
            item_action.try_use(inv, ItemTargetPosition::Entity(entity));
        }
    }
}

pub fn player_drop_item(
    mut query: Query<
        (&mut Inventory, &Transform, &Velocity, &ActionState<Action>),
        With<ControlledPlayer>,
    >,
    mut drop: EventWriter<SpawnDroppedItemEvent>,
) {
    for (mut inv, tf, v, action) in query.iter_mut() {
        if action.just_pressed(&Action::DropItem) {
            let slot = inv.selected_slot();
            if let Some(stack) = inv.drop_items(slot, 1) {
                drop.send(SpawnDroppedItemEvent {
                    postion: tf.translation,
                    velocity: v.0 + tf.forward().as_vec3() * 0.2,
                    stack,
                });
            }
        }
    }
}
