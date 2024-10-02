use abilities::{
    dash::{CurrentlyDashing, Dash},
    stamina::Stamina,
};
use bevy::{prelude::*, window::CursorGrabMode};
use ghost::FloatBoost;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    actors::*,
    items::{inventory::Inventory, EquipItemEvent, UnequipItemEvent},
    physics::{
        collision::Aabb,
        grapple::Grappled,
        movement::{GravityMult, Velocity},
        query::{self, Raycast, RaycastHit},
    },
    world::{
        events::{BlockHitEvent, BlockUsedEvent},
        settings::Settings,
        BlockCoord, BlockPhysics, Level, LevelSystemSet, UsableBlock,
    },
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
                player_scroll_inventory,
                toggle_player_flight,
            )
                .in_set(LevelSystemSet::Main),
        )
        .add_systems(Update, update_window_focused)
        .insert_resource(WindowFocused(false));
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
pub struct WindowFocused(pub bool);

fn update_window_focused(mut focused: ResMut<WindowFocused>, query: Query<&Window>) {
    focused.0 = query
        .get_single()
        .map(|w| w.cursor.grab_mode != CursorGrabMode::None)
        .unwrap_or(false);
}

pub fn toggle_player_flight(
    action: Res<ActionState<Action>>,
    mut query: Query<(&mut MovementMode, &mut GravityMult), With<Player>>,
) {
    for (mut mode, mut gravity) in query.iter_mut() {
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
    mut query: Query<(&Transform, &mut TickMovement, &MovementMode), With<Player>>,
    action: Res<ActionState<Action>>,
) {
    for (tf, mut fm, mode) in query.iter_mut() {
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
    mut query: Query<(Entity, &mut FloatBoost, &MovementMode), With<Player>>,
    mut commands: Commands,
    action: Res<ActionState<Action>>,
) {
    for (entity, mut fb, mode) in query.iter_mut() {
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
        (Entity, &Dash, &mut Stamina, &Velocity),
        (With<Player>, Without<CurrentlyDashing>),
    >,
    mut commands: Commands,
    time: Res<Time>,
    action: Res<ActionState<Action>>,
) {
    let current_time = time.elapsed();
    for (entity, dash, mut stamina, v) in query.iter_mut() {
        if action.just_pressed(&Action::Dash) && dash.stamina_cost.apply(&mut stamina) {
            if let Some(mut ec) = commands.get_entity(entity) {
                ec.insert(CurrentlyDashing::new(*dash, current_time, v.0));
            }
        }
    }
}

pub fn rotate_mouse(
    mut query: Query<(&mut Transform, &mut RotateWithMouse)>,
    focused: Res<WindowFocused>,
    settings: Res<Settings>,
    action: Res<ActionState<Action>>,
) {
    if !focused.0 {
        return;
    }
    let sensitivity = settings.mouse_sensitivity;
    for (mut tf, mut rotation) in query.iter_mut() {
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
    mut player_query: Query<(Entity, &GlobalTransform, &Player, &mut Inventory), With<LocalPlayer>>,
    combat_query: Query<&Combatant>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    mut attack_punch_writer: EventWriter<AttackEvent>,
    mut block_hit_writer: EventWriter<BlockHitEvent>,
    focused: Res<WindowFocused>,
    level: Res<Level>,
    action: Res<ActionState<Action>>,
) {
    if !focused.0 {
        return;
    }
    if let Ok((player_entity, tf, player, mut inv)) = player_query.get_single_mut() {
        if action.pressed(&Action::Punch) {
            //first test if we punched a combatant
            let slot = inv.selected_slot();
            match inv.get(slot) {
                Some(_) => inv.swing_item(
                    slot,
                    crate::items::inventory::ItemTargetPosition::Entity(player_entity),
                ),
                None => {
                    //todo convert to ability
                    match query::raycast(
                        Raycast::new(tf.translation(), tf.forward(), 10.0),
                        &level,
                        &block_physics_query,
                        &object_query,
                        &[player_entity],
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
                                    knockback: *tf.forward(),
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
    mut player_query: Query<(&mut Inventory, Entity, &GlobalTransform), With<LocalPlayer>>,
    focused: Res<WindowFocused>,
    level: Res<Level>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
    usable_block_query: Query<&UsableBlock>,
    mut block_use_writer: EventWriter<BlockUsedEvent>,
    action: Res<ActionState<Action>>,
) {
    if !focused.0 {
        return;
    }
    if let Ok((mut inv, entity, tf)) = player_query.get_single_mut() {
        if action.just_pressed(&Action::Use) {
            //first test if we used a block
            if let Some(RaycastHit::Block(coord, _)) = query::raycast(
                Raycast::new(tf.translation(), tf.forward(), 10.0),
                &level,
                &block_physics_query,
                &object_query,
                &[entity],
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
            inv.use_item(
                slot,
                crate::items::inventory::ItemTargetPosition::Entity(entity),
            );
        }
    }
}

pub fn player_scroll_inventory(
    mut query: Query<&mut Inventory, With<LocalPlayer>>,
    mut equip_writer: EventWriter<EquipItemEvent>,
    mut unequip_writer: EventWriter<UnequipItemEvent>,
    focused: Res<WindowFocused>,
    action: Res<ActionState<Action>>,
) {
    if !focused.0 {
        return;
    }
    const SCROLL_SENSITIVITY: f32 = 0.05;
    if let Ok(mut inv) = query.get_single_mut() {
        let delta = action.value(&Action::Scroll);
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
