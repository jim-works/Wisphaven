use std::time::Duration;

use bevy::{ecs::system::SystemId, prelude::*};

use crate::{
    actors::{process_attacks, CombatInfo, DamageTakenEvent, LocalPlayer, LocalPlayerSpawnedEvent},
    util::inverse_lerp,
    LevelLoadState,
};

use super::state::UIState;

pub struct PlayerStatsUiPlugin;

impl Plugin for PlayerStatsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .add_systems(PostUpdate, flash_hearts.after(process_attacks))
            .add_systems(OnEnter(UIState::Default), show_player_stat_ui)
            .add_systems(OnExit(UIState::Default), hide_player_stat_ui)
            .add_systems(Update, spawn_heart.run_if(in_state(LevelLoadState::Loaded)));

        let update_hearts_id = app.world.register_system(update_hearts);
        app.insert_resource(HeartSystems {
            update_hearts: update_hearts_id,
        });
    }
}

#[derive(Resource)]
pub struct PlayerHealthUiResources {
    pub heart: UiImage,
    pub broken_heart: UiImage,
    pub flash_heart: UiImage,
    pub empty_heart: UiImage,
    pub happy_ghost: UiImage,
    pub sad_ghost: UiImage,
    pub heart_style: Style,
    pub heart_overlay_style: Style,
    pub ghost_style: Style,
}

//images
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

//containers
#[derive(Component, Clone, Copy)]
pub struct PlayerStatContainer;

#[derive(Component, Clone, Copy)]
pub struct PlayerHeartContainer;

#[derive(Resource)]
struct HeartSystems {
    update_hearts: SystemId,
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
    commands
        .spawn((
            PlayerStatContainer,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::ColumnReverse,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ..default()
            },
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
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    ..default()
                },
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
    mut heart_query: Query<&mut BackgroundColor, With<PlayerFlashHeart>>,
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
                    heart.0 = Color::rgba(1.0, 1.0, 1.0, 1.0);
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
                heart.0 = Color::rgba(1.0, 1.0, 1.0, 0.0);
            }
        } else {
            //inactive, switch to active
            state.0 = flash_duration;
            state.2 = true;
            for mut heart in heart_query.iter_mut() {
                heart.0 = Color::rgba(1.0, 1.0, 1.0, 1.0);
            }
        }
    }
}

fn update_hearts(
    player_query: Query<&CombatInfo, With<LocalPlayer>>,
    mut heart_query: Query<(&PlayerBrokenHeart, &mut BackgroundColor)>,
) {
    let curr_health = player_query
        .get_single()
        .map_or(0.0, |info| info.curr_health);
    for (
        PlayerBrokenHeart {
            min_health,
            max_health,
        },
        mut color,
    ) in heart_query.iter_mut()
    {
        let progress = inverse_lerp(*max_health, *min_health, curr_health);
        let alpha = progress.clamp(0.0, 1.0);
        color.0 = color.0.with_a(alpha);
    }
}

fn spawn_heart(
    mut commands: Commands,
    mut reader: EventReader<LocalPlayerSpawnedEvent>,
    combat_query: Query<&CombatInfo>,
    res: Res<PlayerHealthUiResources>,
    root_query: Query<Entity, With<PlayerHeartContainer>>,
    systems: Res<HeartSystems>,
) {
    for LocalPlayerSpawnedEvent(entity) in reader.read() {
        if let (Ok(root), Ok(player_combat)) = (root_query.get_single(), combat_query.get(*entity))
        {
            commands.entity(root).with_children(|children| {
                for i in 0..player_combat.max_health.ceil() as i32 {
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
                                        image: res.empty_heart.clone(),
                                        background_color: BackgroundColor(Color::rgba(
                                            1.0, 1.0, 1.0, 0.0,
                                        )),
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
                                            image: res.flash_heart.clone(),
                                            background_color: BackgroundColor(Color::rgba(
                                                1.0, 1.0, 1.0, 0.0,
                                            )),
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
