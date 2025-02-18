use bevy::prelude::*;

use interfaces::scheduling::GameState;
use world::atmosphere::Calendar;

use waves::waves::{ActiveAssault, Assault};

use crate::MainCameraUIRoot;

pub struct WavesPlugin;

impl Plugin for WavesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init).add_systems(
            Update,
            (
                spawn_wave_indicators,
                update_progress_bar,
                despawn_container,
            )
                .chain()
                .run_if(resource_exists::<Calendar>),
        );
    }
}

#[derive(Resource)]
struct WaveUIResources {
    wave_indicator_texture: Handle<Image>,
    root_entity: Entity,
}

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIScreen;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIContainer {
    assault: Entity,
}

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIProgressBarBackground;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIProgressBarForeground;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIProgressLabel;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIWaveIndicatorContainer;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIWaveIndicatorParent(usize);

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIWaveIndicator(usize);

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    let root_entity = commands
        .spawn((
            WaveUIScreen,
            MainCameraUIRoot,
            PickingBehavior::IGNORE,
            Name::new("WaveUIScreen"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                justify_content: JustifyContent::FlexStart,
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id();
    commands.insert_resource(WaveUIResources {
        wave_indicator_texture: asset_server.load("textures/ui/exclamation.png"),
        root_entity,
    });
}

fn spawn_wave_indicators(
    mut commands: Commands,
    assault_query: Query<(Entity, &Assault), Added<ActiveAssault>>,
    resources: Res<WaveUIResources>,
    calendar: Res<Calendar>,
) {
    for (assault_entity, assault) in assault_query.iter() {
        let margin = UiRect::all(Val::Px(2.));
        commands
            .entity(resources.root_entity)
            .with_children(|container| {
                container
                    .spawn((
                        WaveUIContainer {
                            assault: assault_entity,
                        },
                        Node {
                            width: Val::Px(240.0),
                            height: Val::Px(16.0),
                            margin,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexEnd,
                            justify_content: JustifyContent::Center,
                            position_type: PositionType::Relative,
                            ..default()
                        },
                        Visibility::Visible,
                        StateScoped(GameState::Game),
                    ))
                    .with_children(|children| {
                        // progress bar & background
                        children
                            .spawn((
                                WaveUIProgressBarBackground,
                                Node {
                                    width: Val::Percent(100.),
                                    height: Val::Px(12.0),
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::FlexEnd,
                                    ..default()
                                },
                                BackgroundColor(Color::Srgba(Srgba::hex("202e37").unwrap())),
                            ))
                            .with_children(|bar| {
                                bar.spawn((
                                    WaveUIProgressBarForeground,
                                    Node {
                                        width: Val::Percent(0.),
                                        height: Val::Percent(100.),
                                        position_type: PositionType::Absolute,
                                        ..default()
                                    },
                                    BackgroundColor(Color::Srgba(Srgba::hex("4f8fba").unwrap())),
                                ));
                            });
                        children.spawn((
                            WaveUIWaveIndicatorContainer,
                            Node {
                                width: Val::Percent(100.),
                                height: Val::Percent(100.),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                        ));
                        // wave indicators
                        for (i, wave) in assault.waves.iter().enumerate() {
                            if !wave.visible {
                                continue;
                            }
                            let progress = 100.
                                * (wave.start_time.as_secs_f32()
                                    - calendar.day_length.as_secs_f32())
                                / calendar.night_length.as_secs_f32();
                            children
                                .spawn((
                                    WaveUIWaveIndicatorParent(i),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(16.),
                                        height: Val::Px(16.),
                                        left: Val::Percent(progress),
                                        ..default()
                                    },
                                ))
                                .with_children(|indicator| {
                                    indicator.spawn((
                                        WaveUIWaveIndicator(i),
                                        Name::new("wave indicator"),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: Val::Px(16.),
                                            height: Val::Px(16.),
                                            left: Val::Px(-8.),
                                            top: Val::Px(2.),
                                            ..default()
                                        },
                                        ImageNode::new(resources.wave_indicator_texture.clone()),
                                    ));
                                });
                        }
                    });
            });
    }
}

fn update_progress_bar(
    mut fill_query: Query<&mut Node, With<WaveUIProgressBarForeground>>,
    calendar: Res<Calendar>,
) {
    let night_progress = (calendar.get_sun_progress() - 0.5) * 2.;
    for mut style in fill_query.iter_mut() {
        style.width = Val::Percent(100. * night_progress);
    }
}

fn despawn_container(
    ui_query: Query<(Entity, &WaveUIContainer)>,
    assault_query: Query<(), With<ActiveAssault>>,
    mut commands: Commands,
) {
    for (ui_entity, container) in ui_query.iter() {
        if !assault_query.contains(container.assault) {
            commands.entity(ui_entity).despawn_recursive();
        }
    }
}
