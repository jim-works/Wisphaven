use bevy::prelude::*;

use super::styles::get_small_text_style;

pub struct WavesPlugin;

impl Plugin for WavesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
    }
}

#[derive(Component, Clone, Copy)]
struct WaveUIScreen;

#[derive(Component, Clone, Copy)]
struct WaveUIContainer;

#[derive(Component, Clone, Copy)]
struct WaveUIProgressBarBackground;

#[derive(Component, Clone, Copy)]
struct WaveUIProgressBarForeground;

#[derive(Component, Clone, Copy)]
struct WaveUIProgressLabel;

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            WaveUIScreen,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    justify_content: JustifyContent::FlexStart,
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|container| {
            container
                .spawn((
                    WaveUIContainer,
                    NodeBundle {
                        style: Style {
                            width: Val::Px(240.0),
                            height: Val::Px(32.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexEnd,
                            justify_content: JustifyContent::Center,
                            position_type: PositionType::Relative,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::rgba(0.3, 0.3, 0.3, 0.5)),
                        visibility: Visibility::Visible,
                        ..default()
                    },
                ))
                .with_children(|children| {
                    children
                        .spawn((
                            WaveUIProgressBarBackground,
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.),
                                    height: Val::Px(16.0),
                                    justify_content: JustifyContent::FlexEnd,
                                    align_items: AlignItems::FlexEnd,
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::hex("202e37").unwrap()),
                                ..default()
                            },
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                WaveUIProgressBarForeground,
                                NodeBundle {
                                    style: Style {
                                        width: Val::Percent(50.),
                                        height: Val::Percent(100.),
                                        position_type: PositionType::Absolute,
                                        ..default()
                                    },
                                    background_color: BackgroundColor(
                                        Color::hex("4f8fba").unwrap(),
                                    ),
                                    ..default()
                                },
                            ));
                        });
                    children.spawn((
                        WaveUIProgressLabel,
                        TextBundle {
                            style: Style {
                                width: Val::Percent(100.),
                                height: Val::Px(16.0),
                                ..default()
                            },
                            text: Text {
                                sections: vec![TextSection::new(
                                    "Enemies Remaining: 5",
                                    get_small_text_style(&asset_server),
                                )],
                                alignment: TextAlignment::Right,
                                ..default()
                            },
                            ..default()
                        },
                    ));
                });
        });
}
