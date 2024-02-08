use std::time::Duration;

use bevy::prelude::*;

use crate::{
    actors::{process_attacks, DamageTakenEvent, LocalPlayer},
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
            .add_systems(OnEnter(LevelLoadState::Loaded), spawn_heart);
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
#[derive(Component)]
struct PlayerHeart;

#[derive(Component)]
struct PlayerBrokenHeart;

#[derive(Component)]
struct PlayerFlashHeart;

#[derive(Component)]
struct PlayerEmptyHeart;

#[derive(Component)]
struct PlayerHappyGhost;

#[derive(Component)]
struct PlayerSadGhost;

//containers
#[derive(Component)]
pub struct PlayerStatContainer;

#[derive(Component)]
pub struct PlayerHeartContainer;

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

fn spawn_heart(
    mut commands: Commands,
    res: Res<PlayerHealthUiResources>,
    root_query: Query<Entity, With<PlayerHeartContainer>>,
) {
    if let Ok(root) = root_query.get_single() {
        commands.entity(root).with_children(|children| {
            for _ in 0..10 {
                info!("spawned heart");
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
                                    image: res.broken_heart.clone(),
                                    background_color: BackgroundColor(Color::rgba(
                                        1.0, 1.0, 1.0, 0.0,
                                    )),
                                    ..default()
                                },
                                PlayerBrokenHeart,
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
}
