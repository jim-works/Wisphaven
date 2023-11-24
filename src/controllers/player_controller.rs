use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    actors::*,
    items::{inventory::Inventory, EquipItemEvent, UnequipItemEvent},
    ui::{state::UIState, world_mouse_active},
    world::{
        events::{BlockHitEvent, BlockUsedEvent},
        BlockCoord, Level, UsableBlock,
    },
};

use super::{Action, FrameJump, FrameMovement};

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

pub fn move_player(mut query: Query<(&ActionState<Action>, &Transform, &mut FrameMovement), With<Player>>) {
    for (act, tf, mut fm) in query.iter_mut() {
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
        fm.0 = tf.rotation*dv;
    }
}

//TODO: extract most of this into another system to look like move_player and do_planar_movement
pub fn jump_player(mut query: Query<(&mut FrameJump, &ActionState<Action>), With<Player>>) {
    for (mut fj, act) in query.iter_mut() {
        if act.just_pressed(Action::Jump) {
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
    mut attack_punch_writer: EventWriter<AttackEvent>,
    mut block_hit_writer: EventWriter<BlockHitEvent>,
    collision: Res<RapierContext>,
    ui_state: Res<State<UIState>>,
) {
    if !world_mouse_active(ui_state.get()) {
        return;
    }
    if let Ok((tf, act)) = camera_query.get_single() {
        if act.pressed(Action::Punch) {
            let (player_entity, player, mut inv) = player_query.get_single_mut().unwrap();
            //first test if we punched a combatant
            let groups = QueryFilter {
                groups: Some(CollisionGroups::new(
                    Group::ALL,
                    Group::from_bits_truncate(crate::physics::ACTOR_GROUP),
                )),
                ..default()
            }
            .exclude_collider(player_entity);
            let slot = inv.selected_slot();
            match inv.get(slot) {
                Some(_) => inv.swing_item(slot, *tf),
                None => {
                    if let Some((hit, t)) =
                        collision.cast_ray(tf.translation(), tf.forward(), 10.0, true, groups)
                    {
                        //add 0.05 to move a bit into the block
                        let hit_pos = tf.translation() + tf.forward() * (t + 0.05);
                        //TODO: convert to ability
                        if combat_query.contains(hit) {
                            attack_punch_writer.send(AttackEvent {
                                attacker: player_entity,
                                target: hit,
                                damage: player.hit_damage,
                                knockback: tf.forward(),
                            })
                        } else {
                            block_hit_writer.send(BlockHitEvent {
                                item: None,
                                user: Some(player_entity),
                                block_position: BlockCoord::from(hit_pos),
                                hit_forward: tf.forward(),
                            });
                        }
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
                if let Some(hit) = level.blockcast(tf.translation(), tf.forward() * 10.0) {
                    if level.use_block(
                        hit.block_pos,
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
