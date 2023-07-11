use std::f32::consts::PI;

use bevy::{prelude::*, render::{primitives::Frustum, camera::CameraProjection}};
use bevy_atmosphere::prelude::AtmosphereCamera;
use bevy_rapier3d::prelude::*;
use leafwing_input_manager::InputManagerBundle;

use crate::{world::{*, settings::Settings}, controllers::{*, self}, physics::*, items::{inventory::Inventory, *}};

use super::{CombatantBundle, CombatInfo, DeathInfo, Jump};

#[derive(Component)]
pub struct Player{
    pub hit_damage: f32,
}

#[derive(Component)]
pub struct LocalPlayer;

pub struct LocalPlayerSpawnedEvent(pub Entity);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_local_player.in_schedule(OnEnter(LevelLoadState::Loaded)))
        .add_event::<LocalPlayerSpawnedEvent>();
    }
}

pub fn spawn_local_player(
    mut commands: Commands,
    settings: Res<Settings>,
    level: Res<Level>,
    mut pickup_item: EventWriter<PickupItemEvent>,
    mut equip_item: EventWriter<EquipItemEvent>,
    mut spawn_event: EventWriter<LocalPlayerSpawnedEvent>,
    resources: Res<ItemResources>,
    item_query: Query<&MaxStackSize>,
) {
    info!("Spawning local player!");
    let player_id = commands.spawn((
        Name::new("local player"),
        Player {hit_damage: 1.0},
        LocalPlayer {},
        CombatantBundle {
            combat_info: CombatInfo::new(10.0, 0.0),
            death_info: DeathInfo { death_type: crate::actors::DeathType::LocalPlayer}
        },
        RotateWithMouse {
            lock_pitch: true,
            ..default()
        },
        TransformBundle::from_transform(Transform::from_translation(level.spawn_point)),
        ControllableBundle {
            physics: PhysicsObjectBundle {
                collision_groups: CollisionGroups::new(
                    Group::from_bits_truncate(PLAYER_GROUP | ACTOR_GROUP),
                    Group::all(),
                ),
                ..default()
            },
            ..default()
        },
        Jump::default(),
        settings.player_loader.clone(),
        InputManagerBundle {
            input_map: controllers::get_input_map(),
            ..default()
        },
        )).id();
        let mut inventory = Inventory::new(player_id, 40);

        inventory.pickup_item(ItemStack::new(resources.registry.get_basic(&ItemName::core("grass_block")).unwrap(),100), &item_query, &mut pickup_item, &mut equip_item);
        inventory.pickup_item(ItemStack::new(resources.registry.get_basic(&ItemName::core("tnt_block")).unwrap(),100), &item_query, &mut pickup_item, &mut equip_item);
        inventory.pickup_item(ItemStack::new(resources.registry.get_basic(&ItemName::core("personality_tester")).unwrap(),100), &item_query, &mut pickup_item, &mut equip_item);
        inventory.pickup_item(ItemStack::new(resources.registry.get_basic(&ItemName::core("dagger")).unwrap(),100), &item_query, &mut pickup_item, &mut equip_item);
        inventory.pickup_item(ItemStack::new(resources.registry.get_basic(&ItemName::core("mega_air")).unwrap(),100), &item_query, &mut pickup_item, &mut equip_item);

        commands.entity(player_id).insert(inventory);
        let projection = PerspectiveProjection {
            fov: PI / 2.,
            far: 1_000_000_000.0,
            ..default()
        };
        spawn_event.send(LocalPlayerSpawnedEvent(player_id));
        commands.spawn((
            Name::new("Camera"),
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 1.5, 0.0),
                projection: Projection::Perspective(projection.clone()),
                frustum: Frustum::from_view_projection(&projection.get_projection_matrix()),
                ..default()
            },
            AtmosphereCamera::default(),
            FogSettings {
                color: Color::rgba(1.0, 1.0, 1.0, 0.5),
                falloff: FogFalloff::from_visibility_colors(
                    500.0, // distance in world units up to which objects retain visibility (>= 5% contrast)
                    Color::rgba(0.35, 0.5, 0.5, 0.5), // atmospheric extinction color (after light is lost due to absorption by atmospheric particles)
                    Color::rgba(0.8, 0.844, 1.0, 0.5), // atmospheric inscattering color (light gained due to scattering from the sun)
                ),
                ..default()
            },
            RotateWithMouse {
                pitch_bound: PI * 0.49,
                lock_yaw: true,
                ..default()
            },
            FollowPlayer {},
            PlayerActionOrigin {},
            InputManagerBundle {
                input_map: controllers::get_input_map(),
                ..default()
            },
        ));
}