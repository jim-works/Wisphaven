use bevy::prelude::*;

use super::state::UIState;

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .add_systems(OnEnter(UIState::Default), spawn_crosshair)
            .add_systems(OnExit(UIState::Default), despawn_crosshair)
        ;
    }
}

#[derive(Resource)]
struct CrosshairResources(UiImage, Style);

#[derive(Component)]
struct Crosshair;

fn init(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(CrosshairResources(
        assets.load("textures/crosshair.png").into(),
        Style {
            width: Val::Px(16.0),
            height: Val::Px(16.0),
            aspect_ratio: Some(1.0),
            ..default()
        }
    ));
}

fn spawn_crosshair(
    mut commands: Commands,
    mut query: Query<&mut Visibility, With<Crosshair>>,
    resources: Res<CrosshairResources>
) {
    if let Ok(mut crosshair) = query.get_single_mut() {
        *crosshair.as_mut() = Visibility::Inherited;
    } else {
        commands.spawn((Crosshair, NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            ..default()
        })
    ).with_children(|children| {children.spawn(ImageBundle {
        style: resources.1.clone(),
        image: resources.0.clone(),
        ..default()
    });});
    }
}

fn despawn_crosshair(
    mut query: Query<&mut Visibility, With<Crosshair>>
) {
    if let Ok(mut crosshair) = query.get_single_mut() {
        *crosshair.as_mut() = Visibility::Hidden;
    } else {
        warn!("Tried to despawn crosshair when one doesn't exist!");
    }
}