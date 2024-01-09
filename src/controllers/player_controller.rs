use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    actors::*,
    items::{inventory::Inventory, EquipItemEvent, UnequipItemEvent},
    physics::{
        collision::Aabb,
        movement::GravityMult,
        query::{self, Ray, RaycastHit},
    },
    ui::{state::UIState, world_mouse_active},
    world::{
        events::{BlockHitEvent, BlockUsedEvent},
        BlockCoord, BlockPhysics, Level, UsableBlock,
    },
};

use super::{Action, FrameJump, FrameMovement, MovementMode};

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
pub struct FollowPlayer {}

#[derive(Component)]
pub struct PlayerActionOrigin {}

pub fn toggle_player_flight(
    mut query: Query<(&ActionState<Action>, &mut MovementMode, &mut GravityMult), With<Player>>,
) {
    for (act, mut mode, mut gravity) in query.iter_mut() {
        if act.just_pressed(Action::ToggleFlight) {
            match *mode {
                MovementMode::Flying => {
                    *mode = MovementMode::Normal;
                    gravity.0 = 1.0;
                    info!("Not Flying");
                }
                _ => {
                    *mode = MovementMode::Flying;
                    gravity.0 = 0.0;
                    info!("Flying");
                }
            };
        }
    }
}

pub fn move_player(
    mut query: Query<
        (
            &ActionState<Action>,
            &Transform,
            &mut FrameMovement,
            &MovementMode,
        ),
        With<Player>,
    >,
) {
    for (act, tf, mut fm, mode) in query.iter_mut() {
        let mut dv = Vec3::ZERO;
        dv.z -= if act.pressed(Action::MoveForward) {
            1.0
        } else {
            0.0
        };
        dv.z += if act.pressed(Action::MoveBack) {
            1.0
        } else {
            0.0
        };
        dv.x += if act.pressed(Action::MoveRight) {
            1.0
        } else {
            0.0
        };
        dv.x -= if act.pressed(Action::MoveLeft) {
            1.0
        } else {
            0.0
        };

        fm.0 = tf.rotation * dv;
        if *mode == MovementMode::Flying {
            fm.0.y += if act.pressed(Action::MoveUp) {
                1.0
            } else {
                0.0
            };
            fm.0.y -= if act.pressed(Action::MoveDown) {
                1.0
            } else {
                0.0
            };
        }
    }
}

pub fn jump_player(
    mut query: Query<(&mut FrameJump, &ActionState<Action>, &MovementMode), With<Player>>,
) {
    for (mut fj, act, mode) in query.iter_mut() {
        if *mode != MovementMode::Flying && act.just_pressed(Action::Jump) {
            fj.0 = true;
        }
    }
}

pub fn rotate_mouse(
    mut query: Query<(&mut Transform, &mut RotateWithMouse, &ActionState<Action>)>,
    ui_state: Res<State<UIState>>,
) {
    if !world_mouse_active(ui_state.get()) {
        return;
    }
    const SENSITIVITY: f32 = 0.01;
    for (mut tf, mut rotation, action) in query.iter_mut() {
        if let Some(delta) = action.axis_pair(Action::Look) {
            if !rotation.lock_yaw {
                rotation.yaw -= delta.x() * SENSITIVITY;
            }
            if !rotation.lock_pitch {
                rotation.pitch -= delta.y() * SENSITIVITY;
            }

            rotation.pitch = rotation
                .pitch
                .clamp(-rotation.pitch_bound, rotation.pitch_bound);

            tf.rotation = Quat::from_axis_angle(Vec3::Y, rotation.yaw)
                * Quat::from_axis_angle(Vec3::X, rotation.pitch)
                * Quat::from_axis_angle(Vec3::Z, rotation.roll);
        }
    }
}

pub fn follow_local_player(
    player_query: Query<(&Transform, &RotateWithMouse), With<LocalPlayer>>,
    mut follow_query: Query<
        (&mut Transform, Option<&mut RotateWithMouse>),
        (With<FollowPlayer>, Without<LocalPlayer>),
    >,
) {
    if let Ok((player_tf, player_rot)) = player_query.get_single() {
        for (mut follow_tf, opt_follow_rot) in follow_query.iter_mut() {
            follow_tf.translation = player_tf.translation + Vec3::new(0.0, 1.5, 0.0);
            if let Some(mut follow_rot) = opt_follow_rot {
                follow_rot.yaw = player_rot.yaw;
            }
        }
    }
}

pub fn player_punch(
    camera_query: Query<
        (&GlobalTransform, &ActionState<Action>),
        (
            With<PlayerActionOrigin>,
            With<FollowPlayer>,
            Without<LocalPlayer>,
        ),
    >,
    mut player_query: Query<(Entity, &Player, &mut Inventory), With<LocalPlayer>>,
    combat_query: Query<&CombatInfo>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    mut attack_punch_writer: EventWriter<AttackEvent>,
    mut block_hit_writer: EventWriter<BlockHitEvent>,
    ui_state: Res<State<UIState>>,
    level: Res<Level>,
) {
    if !world_mouse_active(ui_state.get()) {
        return;
    }
    if let Ok((tf, act)) = camera_query.get_single() {
        if act.pressed(Action::Punch) {
            let (player_entity, player, mut inv) = player_query.get_single_mut().unwrap();
            //first test if we punched a combatant
            let slot = inv.selected_slot();
            match inv.get(slot) {
                Some(_) => inv.swing_item(slot, *tf),
                None => {
                    //todo convert to ability
                    match query::raycast(
                        Ray::new(tf.translation(), tf.forward(), 10.0),
                        &level,
                        &block_physics_query,
                        &object_query,
                        vec![player_entity]
                    ) {
                        Some(RaycastHit::Block(hit_pos, _)) => {
                            block_hit_writer.send(BlockHitEvent {
                                item: None,
                                user: Some(player_entity),
                                block_position: BlockCoord::from(hit_pos),
                                hit_forward: tf.forward(),
                            });
                        }
                        Some(RaycastHit::Object(hit)) => {
                            if combat_query.contains(hit.entity) {
                                attack_punch_writer.send(AttackEvent {
                                    attacker: player_entity,
                                    target: hit.entity,
                                    damage: player.hit_damage,
                                    knockback: tf.forward(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

pub fn player_use(
    camera_query: Query<
        (&GlobalTransform, &ActionState<Action>),
        (
            With<PlayerActionOrigin>,
            With<FollowPlayer>,
            Without<LocalPlayer>,
        ),
    >,
    mut player_query: Query<(&mut Inventory, Entity), With<LocalPlayer>>,
    ui_state: Res<State<UIState>>,
    level: Res<Level>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    usable_block_query: Query<&UsableBlock>,
    mut block_use_writer: EventWriter<BlockUsedEvent>,
) {
    if !world_mouse_active(ui_state.get()) {
        return;
    }
    if let Ok((mut inv, entity)) = player_query.get_single_mut() {
        if let Ok((tf, act)) = camera_query.get_single() {
            if act.just_pressed(Action::Use) {
                //first test if we used a block
                if let Some(RaycastHit::Block(coord, _)) = query::raycast(
                    Ray::new(tf.translation(), tf.forward(), 10.0),
                    &level,
                    &block_physics_query,
                    &object_query,
                    vec![entity]
                ) {
                    if level.use_block(
                        coord,
                        entity,
                        tf.forward(),
                        &usable_block_query,
                        &mut block_use_writer,
                    ) {
                        //we used a block, so don't also use an item
                        return;
                    }
                }
                //we didn't use a block, so try to use an item
                let slot = inv.selected_slot();
                inv.use_item(slot, *tf);
            }
        }
    }
}

pub fn player_scroll_inventory(
    mut query: Query<(&mut Inventory, &ActionState<Action>), With<LocalPlayer>>,
    mut equip_writer: EventWriter<EquipItemEvent>,
    mut unequip_writer: EventWriter<UnequipItemEvent>,
    ui_state: Res<State<UIState>>,
) {
    if !world_mouse_active(ui_state.get()) {
        return;
    }
    const SCROLL_SENSITIVITY: f32 = 0.05;
    if let Ok((mut inv, act)) = query.get_single_mut() {
        let delta = act.value(Action::Scroll);
        let slot_diff = if delta > SCROLL_SENSITIVITY {
            -1
        } else if delta < -SCROLL_SENSITIVITY {
            1
        } else {
            0
        };
        let curr_slot = inv.selected_slot();
        inv.select_slot(
            curr_slot as i32 + slot_diff,
            &mut equip_writer,
            &mut unequip_writer,
        );
    }
}
