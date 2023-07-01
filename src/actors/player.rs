use std::f32::consts::PI;

use bevy::{prelude::*, render::{primitives::Frustum, camera::CameraProjection}};
use bevy_atmosphere::prelude::AtmosphereCamera;
use bevy_rapier3d::prelude::*;
use leafwing_input_manager::InputManagerBundle;

use crate::{world::{*, settings::Settings}, controllers::{*, self}, physics::*, items::{inventory::Inventory, *, self, block_item::*, weapons::MeleeWeaponItem, debug_items::PersonalityTester}};

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
    item_query: Query<&Item>,
    assets: Res<AssetServer>
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
        let grass_item = items::create_item(Item::new("Grass Block", 999), ItemIcon(assets.load("textures/items/grass_block.png")), BlockItem(BlockType::Basic(0)), &mut commands);
        let personality_tester = items::create_item(Item::new("Personality Tester", 999), ItemIcon(assets.load("textures/items/personality_tester.png")), PersonalityTester, &mut commands);
        let mega_air_item = items::create_item(Item::new("Mega Air", 999), ItemIcon(assets.load("textures/items/vacuum.png")), MegablockItem(BlockType::Empty,10), &mut commands);
        let dagger_item = items::create_item(Item::new("Dagger", 999), ItemIcon(assets.load("textures/items/dagger.png")), MeleeWeaponItem{damage: 5.0, knockback: 2.0}, &mut commands);
        inventory.pickup_item(ItemStack::new(grass_item,1), &item_query, &mut pickup_item, &mut equip_item);
        inventory.pickup_item(ItemStack::new(personality_tester,1), &item_query, &mut pickup_item, &mut equip_item);
        inventory.pickup_item(ItemStack::new(mega_air_item,1), &item_query, &mut pickup_item, &mut equip_item);
        inventory.pickup_item(ItemStack::new(dagger_item,1), &item_query, &mut pickup_item, &mut equip_item);
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