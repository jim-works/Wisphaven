use bevy::prelude::*;

use engine::{world::atmosphere::Calendar, GameState};

use waves::waves::{Assault, AssaultStartedEvent};

use crate::MainCameraUIRoot;

pub struct WavesPlugin;

impl Plugin for WavesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .add_systems(OnEnter(GameState::Game), spawn_ui)
            .add_systems(
                Update,
                (
                    spawn_wave_indicators,
                    update_ui_visibility,
                    update_progress_bar,
                )
                    .run_if(resource_exists::<Assault>)
                    .run_if(resource_exists::<Calendar>),
            );
    }
}

#[derive(Resource)]
struct WaveUIResources {
    wave_indicator_texture: Handle<Image>,
}

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIScreen;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WaveUIContainer;

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
    commands.insert_resource(WaveUIResources {
        wave_indicator_texture: asset_server.load("textures/ui/exclamation.png"),
    });
}

fn spawn_ui(mut commands: Commands) {
    let margin = UiRect::all(Val::Px(2.));
    commands
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
            StateScoped(GameState::Game),
        ))
        .with_children(|container| {
            container
                .spawn((
                    WaveUIContainer,
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
                ))
                .with_children(|children| {
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
                });
        });
}

fn spawn_wave_indicators(
    mut commands: Commands,
    mut assault_event: EventReader<AssaultStartedEvent>,
    assault: Res<Assault>,
    resources: Res<WaveUIResources>,
    calendar: Res<Calendar>,
    container_query: Query<Entity, With<WaveUIProgressBarBackground>>,
) {
    if assault_event.is_empty() {
        return;
    }
    assault_event.clear();

    for container in container_query.iter() {
        let Some(mut ec) = commands.get_entity(container) else {
            continue;
        };
        ec.with_children(|children| {
            for (i, wave) in assault.to_spawn.iter().enumerate() {
                if !wave.visible {
                    continue;
                }
                let progress = 100.
                    * (wave.start_time.as_secs_f32() - calendar.day_length.as_secs_f32())
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

fn update_ui_visibility(
    mut visibility_query: Query<&mut Visibility, With<WaveUIContainer>>,
    calendar: Res<Calendar>,
    mut assault_event: EventReader<AssaultStartedEvent>,
) {
    let update_visibility = calendar.in_day() || !assault_event.is_empty();
    let visibility = match calendar.in_day() {
        true => Visibility::Hidden,
        false => Visibility::Inherited,
    };
    assault_event.clear();
    for mut v in visibility_query.iter_mut() {
        if update_visibility {
            *v = visibility;
        }
    }
}
