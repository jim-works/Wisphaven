use std::time::Duration;

use bevy::{ecs::system::SystemId, prelude::*};

use engine::{
    actors::{
        abilities::{
            stamina::send_stamina_updated_events, stamina::Stamina, stamina::StaminaUpdatedEvent,
        },
        damage::process_attacks,
        Combatant, DamageTakenEvent, LocalPlayer, LocalPlayerSpawnedEvent,
    },
    world::LevelLoadState,
};

use util::inverse_lerp;

use crate::MainCameraUIRoot;

use super::state::UIState;

pub struct PlayerStatsUiPlugin;

impl Plugin for PlayerStatsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .add_systems(
                PostUpdate,
                (
                    flash_hearts.after(process_attacks),
                    flash_stamina.after(send_stamina_updated_events),
                ),
            )
            .add_systems(OnEnter(UIState::Default), show_player_stat_ui)
            .add_systems(OnExit(UIState::Default), hide_player_stat_ui)
            .add_systems(
                Update,
                (spawn_heart, spawn_stamina).run_if(in_state(LevelLoadState::Loaded)),
            );

        let update_hearts_id = app.world_mut().register_system(update_hearts);
        app.insert_resource(HeartSystems {
            update_hearts: update_hearts_id,
        });
        let update_stamina_id = app.world_mut().register_system(update_stamina);
        app.insert_resource(StaminaSystems {
            update_stamina: update_stamina_id,
        });
    }
}

#[derive(Resource)]
struct PlayerHealthUiResources {
    heart: UiImage,
    broken_heart: UiImage,
    flash_heart: UiImage,
    empty_heart: UiImage,
    happy_ghost: UiImage,
    sad_ghost: UiImage,
    heart_style: Style,
    heart_overlay_style: Style,
    ghost_style: Style,
}

#[derive(Resource)]
struct PlayerStaminaUiResources {
    bolt: UiImage,
    empty_bolt: UiImage,
    flash_bolt: UiImage,
    style: Style,
    overlay_style: Style,
}

//images

//health
#[derive(Component, Clone, Copy)]
struct PlayerHeart;

#[derive(Component, Clone, Copy)]
struct PlayerBrokenHeart {
    min_health: f32,
    max_health: f32,
}

#[derive(Component, Clone, Copy)]
struct PlayerFlashHeart;

#[derive(Component, Clone, Copy)]
struct PlayerEmptyHeart;

#[derive(Component, Clone, Copy)]
struct PlayerHappyGhost;

#[derive(Component, Clone, Copy)]
struct PlayerSadGhost;

//stamina
#[derive(Component, Clone, Copy)]
struct PlayerEmptyBolt {
    min_stamina: f32,
    max_stamina: f32,
}

#[derive(Component, Clone, Copy)]
struct PlayerFlashBolt;

#[derive(Component, Clone, Copy)]
struct PlayerBolt;

//containers
#[derive(Component, Clone, Copy)]
pub struct PlayerStatContainer;

#[derive(Component, Clone, Copy)]
pub struct PlayerHeartContainer;

#[derive(Component, Clone, Copy)]
pub struct PlayerStaminaContainer;

//systems
#[derive(Resource)]
struct HeartSystems {
    update_hearts: SystemId,
}

#[derive(Resource)]
struct StaminaSystems {
    update_stamina: SystemId,
}

fn init(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(PlayerHealthUiResources {
        heart: assets.load("textures/ui/heart.png").into(),
        broken_heart: assets.load("textures/ui/broken_heart.png").into(),
        flash_heart: assets.load("textures/ui/heart_flash.png").into(),
        empty_heart: assets.load("textures/ui/empty_heart.png").into(),
        happy_ghost: assets.load("textures/ghosts/happy_ghost.png").into(),
        sad_ghost: assets.load("textures/ghosts/sad_ghost.png").into(),
        heart_style: Style {
            width: Val::Px(16.0),
            height: Val::Px(16.0),
            aspect_ratio: Some(1.0),
            margin: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        heart_overlay_style: Style {
            width: Val::Px(16.0),
            height: Val::Px(16.0),
            aspect_ratio: Some(1.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        ghost_style: Style {
            width: Val::Px(32.0),
            height: Val::Px(32.0),
            aspect_ratio: Some(1.0),
            ..default()
        },
    });
    commands.insert_resource(PlayerStaminaUiResources {
        bolt: assets.load("textures/ui/bolt.png").into(),
        flash_bolt: assets.load("textures/ui/flash_bolt.png").into(),
        empty_bolt: assets.load("textures/ui/dead_bolt.png").into(),
        style: Style {
            width: Val::Px(11.0),
            height: Val::Px(16.0),
            aspect_ratio: Some(1.0),
            margin: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        overlay_style: Style {
            width: Val::Px(11.0),
            height: Val::Px(16.0),
            aspect_ratio: Some(1.0),
            position_type: PositionType::Absolute,
            ..default()
        },
    });
    commands
        .spawn((
            PlayerStatContainer,
            MainCameraUIRoot,
            NodeBundle {
                style: Style {
                    min_width: Val::Percent(100.0),
                    min_height: Val::Percent(100.0),
                    flex_direction: FlexDirection::ColumnReverse,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },
                ..default()
            },
            Name::new("UI stat container"),
        ))
        .with_children(|children| {
            children.spawn((
                PlayerHeartContainer,
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(18.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                },
                Name::new("UI heart container"),
            ));
            children.spawn((
                PlayerStaminaContainer,
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(18.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                },
                Name::new("UI stamina container"),
            ));
        });
}

fn show_player_stat_ui(mut query: Query<&mut Visibility, With<PlayerStatContainer>>) {
    for mut vis in query.iter_mut() {
        *vis.as_mut() = Visibility::Inherited;
    }
}

fn hide_player_stat_ui(mut query: Query<&mut Visibility, With<PlayerStatContainer>>) {
    for mut vis in query.iter_mut() {
        *vis.as_mut() = Visibility::Hidden;
    }
}

fn flash_hearts(
    player_query: Query<Entity, With<LocalPlayer>>,
    mut heart_query: Query<&mut UiImage, With<PlayerFlashHeart>>,
    mut reader: EventReader<DamageTakenEvent>,
    mut state: Local<(Duration, i32, bool)>,
    time: Res<Time>,
    mut commands: Commands,
    systems: Res<HeartSystems>,
) {
    let flash_duration = Duration::from_secs_f32(0.1);
    let flashes = 1;
    state.0 = state.0.saturating_sub(time.delta());
    if let Ok(player_entity) = player_query.get_single() {
        for event in reader.read() {
            if event.target == player_entity {
                state.0 = flash_duration;
                state.1 = flashes;
                state.2 = true;
                for mut heart in heart_query.iter_mut() {
                    heart.color = Color::srgba(1.0, 1.0, 1.0, 1.0);
                }
                commands.run_system(systems.update_hearts);
            }
        }
    }

    if state.1 > 0 && state.0.is_zero() {
        //switch color
        if state.2 {
            //active, switch to inactive
            state.0 = flash_duration;
            state.1 -= 1;
            state.2 = false;
            for mut heart in heart_query.iter_mut() {
                heart.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
            }
        } else {
            //inactive, switch to active
            state.0 = flash_duration;
            state.2 = true;
            for mut heart in heart_query.iter_mut() {
                heart.color = Color::srgba(1.0, 1.0, 1.0, 1.0);
            }
        }
    }
}

fn flash_stamina(
    player_query: Query<Entity, With<LocalPlayer>>,
    mut bolt_query: Query<&mut UiImage, With<PlayerFlashBolt>>,
    mut reader: EventReader<StaminaUpdatedEvent>,
    mut state: Local<(Duration, i32, bool)>,
    time: Res<Time>,
    mut commands: Commands,
    systems: Res<StaminaSystems>,
) {
    let flash_duration = Duration::from_secs_f32(0.1);
    let flashes = 1;
    let flash_threshold = 0.25;
    state.0 = state.0.saturating_sub(time.delta());
    if let Ok(player_entity) = player_query.get_single() {
        for StaminaUpdatedEvent {
            entity,
            stamina: _,
            change,
            change_max,
        } in reader.read()
        {
            if *entity == player_entity {
                //flash
                if change.abs() >= flash_threshold || change_max.abs() >= flash_threshold {
                    state.0 = flash_duration;
                    state.1 = flashes;
                    state.2 = true;
                    for mut bolt in bolt_query.iter_mut() {
                        bolt.color = Color::srgba(1.0, 1.0, 1.0, 1.0);
                    }
                }
                //update stats on display
                commands.run_system(systems.update_stamina);
            }
        }
    }

    if state.1 > 0 && state.0.is_zero() {
        //switch color
        if state.2 {
            //active, switch to inactive
            state.0 = flash_duration;
            state.1 -= 1;
            state.2 = false;
            for mut bolt in bolt_query.iter_mut() {
                bolt.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
            }
        } else {
            //inactive, switch to active
            state.0 = flash_duration;
            state.2 = true;
            for mut bolt in bolt_query.iter_mut() {
                bolt.color = Color::srgba(1.0, 1.0, 1.0, 1.0);
            }
        }
    }
}

fn update_hearts(
    player_query: Query<&Combatant, With<LocalPlayer>>,
    mut heart_query: Query<(&PlayerBrokenHeart, &mut UiImage)>,
    combatant_query: Query<&Combatant>,
) {
    let curr_health = player_query.get_single().map_or(0.0, |info| {
        info.get_health(&combatant_query)
            .map_or(0.0, |health| health.current)
    });
    for (
        PlayerBrokenHeart {
            min_health,
            max_health,
        },
        mut image,
    ) in heart_query.iter_mut()
    {
        let progress = inverse_lerp(*max_health, *min_health, curr_health);
        let alpha = progress.clamp(0.0, 1.0);
        image.color = image.color.with_alpha(alpha);
    }
}

fn update_stamina(
    player_query: Query<&Stamina, With<LocalPlayer>>,
    mut bolt_query: Query<(&PlayerEmptyBolt, &mut UiImage)>,
) {
    let curr_stamina = player_query.get_single().map_or(0.0, |info| info.current);
    for (
        PlayerEmptyBolt {
            min_stamina,
            max_stamina,
        },
        mut image,
    ) in bolt_query.iter_mut()
    {
        let progress = inverse_lerp(*max_stamina, *min_stamina, curr_stamina);
        let alpha = progress.clamp(0.0, 1.0);
        image.color = image.color.with_alpha(alpha);
    }
}

fn spawn_heart(
    mut commands: Commands,
    mut reader: EventReader<LocalPlayerSpawnedEvent>,
    combat_query: Query<&Combatant>,
    res: Res<PlayerHealthUiResources>,
    root_query: Query<Entity, With<PlayerHeartContainer>>,
    systems: Res<HeartSystems>,
) {
    for LocalPlayerSpawnedEvent(entity) in reader.read() {
        if let (Ok(root), Ok(player_combat)) = (root_query.get_single(), combat_query.get(*entity))
        {
            commands.entity(root).with_children(|children| {
                for i in 0..player_combat
                    .get_health(&combat_query)
                    .unwrap_or_default()
                    .max
                    .ceil() as i32
                {
                    children
                        .spawn((
                            ImageBundle {
                                style: res.heart_style.clone(),
                                image: res.heart.clone(),
                                ..default()
                            },
                            PlayerHeart,
                        ))
                        .with_children(|heart_overlay| {
                            heart_overlay
                                .spawn((
                                    ImageBundle {
                                        style: res.heart_overlay_style.clone(),
                                        image: res
                                            .empty_heart
                                            .clone()
                                            .with_color(Color::srgba(1.0, 1.0, 1.0, 0.0)),
                                        ..default()
                                    },
                                    PlayerBrokenHeart {
                                        min_health: (i as f32),
                                        max_health: (i as f32 + 1.0),
                                    },
                                ))
                                .with_children(|flash_overlay| {
                                    flash_overlay.spawn((
                                        ImageBundle {
                                            style: res.heart_overlay_style.clone(),
                                            image: res
                                                .flash_heart
                                                .clone()
                                                .with_color(Color::srgba(1.0, 1.0, 1.0, 0.0)),
                                            ..default()
                                        },
                                        PlayerFlashHeart,
                                    ));
                                });
                        });
                }
            });
        }
        commands.run_system(systems.update_hearts);
    }
}

fn spawn_stamina(
    mut commands: Commands,
    mut reader: EventReader<LocalPlayerSpawnedEvent>,
    stamina_query: Query<&Stamina>,
    res: Res<PlayerStaminaUiResources>,
    root_query: Query<Entity, With<PlayerStaminaContainer>>,
    systems: Res<StaminaSystems>,
) {
    for LocalPlayerSpawnedEvent(entity) in reader.read() {
        if let (Ok(root), Ok(stamina)) = (root_query.get_single(), stamina_query.get(*entity)) {
            commands.entity(root).with_children(|children| {
                for i in 0..stamina.max.ceil() as i32 {
                    children
                        .spawn((
                            ImageBundle {
                                style: res.style.clone(),
                                image: res.bolt.clone(),
                                ..default()
                            },
                            PlayerHeart,
                        ))
                        .with_children(|heart_overlay| {
                            heart_overlay
                                .spawn((
                                    ImageBundle {
                                        style: res.overlay_style.clone(),
                                        image: res
                                            .empty_bolt
                                            .clone()
                                            .with_color(Color::srgba(1.0, 1.0, 1.0, 0.0)),
                                        ..default()
                                    },
                                    PlayerEmptyBolt {
                                        min_stamina: (i as f32),
                                        max_stamina: (i as f32 + 1.0),
                                    },
                                ))
                                .with_children(|flash_overlay| {
                                    flash_overlay.spawn((
                                        ImageBundle {
                                            style: res.overlay_style.clone(),
                                            image: res
                                                .flash_bolt
                                                .clone()
                                                .with_color(Color::srgba(1.0, 1.0, 1.0, 0.0)),
                                            ..default()
                                        },
                                        PlayerFlashBolt,
                                    ));
                                });
                        });
                }
            });
        }
        commands.run_system(systems.update_stamina);
    }
}
