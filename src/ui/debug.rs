use bevy::prelude::*;

use crate::{actors::LocalPlayer, world::chunk::ChunkCoord};

use super::{state::DebugUIState, styles::get_text_style};

pub struct DebugUIPlugin;

impl Plugin for DebugUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<DebugUIState>()
            .add_startup_system(init)
            .add_system(spawn_debug.in_schedule(OnEnter(DebugUIState::Shown)))
            .add_system(despawn_debug.in_schedule(OnEnter(DebugUIState::Hidden)))
            .add_systems((update_coords, update_chunk_coords).in_set(OnUpdate(DebugUIState::Shown)))
        ;
    }
}

#[derive(Resource)]
struct DebugResources(TextStyle);

#[derive(Component)]
struct DebugUI;

#[derive(Component)]
struct DebugChunkCoordinates;
#[derive(Component)]
struct DebugCoordinates;

fn init(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(DebugResources(get_text_style(&assets)));
}

fn spawn_debug(mut commands: Commands, query: Query<&DebugUI>, resources: Res<DebugResources>) {
    if query.is_empty() {
        commands
            .spawn((
                DebugUI,
                NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        align_items: AlignItems::FlexEnd,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        ..default()
                    },
                    ..default()
                },
            ))
            .with_children(|children| {
                children.spawn((
                    TextBundle {
                        text: Text {
                            sections: vec![TextSection::new(
                                "test coordinates",
                                resources.0.clone(),
                            )],
                            alignment: TextAlignment::Left,
                            ..default()
                        },
                        ..default()
                    },
                    DebugCoordinates,
                ));
                children.spawn((
                    TextBundle {
                        text: Text {
                            sections: vec![TextSection::new(
                                "test chunk coordinates",
                                resources.0.clone(),
                            )],
                            alignment: TextAlignment::Left,
                            ..default()
                        },
                        ..default()
                    },
                    DebugChunkCoordinates,
                ));
            });
    } else {
        warn!("Tried to spawn debug ui when one already exists!");
    }
}

fn despawn_debug(mut commands: Commands, query: Query<Entity, With<DebugUI>>) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    } else {
        warn!("Tried to despawn debug ui when one doesn't exist!");
    }
}

fn update_coords(
    player_query: Query<&GlobalTransform, With<LocalPlayer>>,
    mut ui_query: Query<&mut Text, With<DebugCoordinates>>,
    resources: Res<DebugResources>
) {
    if let Ok(tf) = player_query.get_single() {
        for mut text in ui_query.iter_mut() {
            if text.sections.is_empty() {
                text.sections = vec![TextSection::default()];
            }
            text.sections[0] = TextSection::new(format!("({:.2}, {:.2}, {:.2})", tf.translation().x, tf.translation().y, tf.translation().z), resources.0.clone());
        }
    }
}

fn update_chunk_coords(
    player_query: Query<&GlobalTransform, With<LocalPlayer>>,
    mut ui_query: Query<&mut Text, With<DebugChunkCoordinates>>,
    resources: Res<DebugResources>
) {
    if let Ok(tf) = player_query.get_single() {
        for mut text in ui_query.iter_mut() {
            if text.sections.is_empty() {
                text.sections = vec![TextSection::default()];
            }
            text.sections[0] = TextSection::new(format!("({:?})", ChunkCoord::from(tf.translation())), resources.0.clone());
        }
    }
}

