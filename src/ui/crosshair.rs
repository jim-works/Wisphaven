use bevy::prelude::*;

use super::state::UIState;

const CROSSHAIR_STYLE: Style = Style {
    size: Size::new(Val::Px(16.0), Val::Px(16.0)),
    aspect_ratio: Some(1.0),
    ..Style::DEFAULT
};

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(init)
            .add_system(spawn_crosshair.in_schedule(OnEnter(UIState::Default)))
            .add_system(despawn_crosshair.in_schedule(OnExit(UIState::Default)))
        ;
    }
}

#[derive(Resource)]
struct CrosshairResources(ImageBundle);

#[derive(Component)]
struct Crosshair;

fn init(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(CrosshairResources(ImageBundle {
        style: CROSSHAIR_STYLE,
        image: assets.load("textures/crosshair.png").into(),
        ..default()
    }));
}

fn spawn_crosshair(
    mut commands: Commands,
    query: Query<&Crosshair>,
    resources: Res<CrosshairResources>
) {
    if query.is_empty() {
        commands.spawn((Crosshair, NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
    ).with_children(|children| {children.spawn(resources.0.clone());});
    info!("\n\nSpawned crosshair\n\n");
    } else {
        warn!("Tried to spawn crosshair when one already exists!");
    }
}

fn despawn_crosshair(
    mut commands: Commands,
    query: Query<Entity, With<Crosshair>>
) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    } else {
        warn!("Tried to despawn crosshair when one doesn't exist!");
    }
}