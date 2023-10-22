use bevy::prelude::*;

use crate::{actors::LocalPlayer, world::chunk::ChunkCoord, worldgen::UsedShaperResources};

use super::{state::DebugUIState, styles::get_text_style};

pub struct DebugUIPlugin;

impl Plugin for DebugUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<DebugUIState>()
            .add_systems(Startup, init)
            .add_systems(OnEnter(DebugUIState::Shown), spawn_debug)
            .add_systems(OnEnter(DebugUIState::Hidden), despawn_debug)
            .add_systems(
                Update,
                (
                    update_coords,
                    update_chunk_coords,
                    update_noises,
                    draw_gizmos,
                )
                    .run_if(in_state(DebugUIState::Shown)),
            );
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
#[derive(Component)]
struct DebugTerrainNoises;

#[derive(Component)]
pub struct DebugDrawTransform;

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
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::FlexEnd,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        position_type: PositionType::Absolute,
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
                children.spawn((
                    TextBundle {
                        text: Text {
                            sections: vec![TextSection::new("test noises", resources.0.clone())],
                            alignment: TextAlignment::Left,
                            ..default()
                        },
                        ..default()
                    },
                    DebugTerrainNoises,
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
    resources: Res<DebugResources>,
) {
    if let Ok(tf) = player_query.get_single() {
        for mut text in ui_query.iter_mut() {
            if text.sections.is_empty() {
                text.sections = vec![TextSection::default()];
            }
            text.sections[0] = TextSection::new(
                format!(
                    "({:.2}, {:.2}, {:.2})",
                    tf.translation().x,
                    tf.translation().y,
                    tf.translation().z
                ),
                resources.0.clone(),
            );
        }
    }
}

fn update_chunk_coords(
    player_query: Query<&GlobalTransform, With<LocalPlayer>>,
    mut ui_query: Query<&mut Text, With<DebugChunkCoordinates>>,
    resources: Res<DebugResources>,
) {
    if let Ok(tf) = player_query.get_single() {
        for mut text in ui_query.iter_mut() {
            if text.sections.is_empty() {
                text.sections = vec![TextSection::default()];
            }
            text.sections[0] = TextSection::new(
                format!("({:?})", ChunkCoord::from(tf.translation())),
                resources.0.clone(),
            );
        }
    }
}

fn update_noises(
    player_query: Query<&GlobalTransform, With<LocalPlayer>>,
    mut ui_query: Query<&mut Text, With<DebugTerrainNoises>>,
    resources: Res<DebugResources>,
    noises: Res<UsedShaperResources>,
) {
    let density_noise = &noises.0.density_noise;
    let heightmap_noise = &noises.0.heightmap_noise;
    let landmass_noise = &noises.0.landmass_noise;
    let squish_noise = &noises.0.squish_noise;
    if let Ok(tf) = player_query.get_single() {
        for mut text in ui_query.iter_mut() {
            if text.sections.is_empty() {
                text.sections = vec![TextSection::default()];
            }
            let pos = tf.translation();
            text.sections[0] = TextSection::new(
                format!(
                    "heightmap: {:.2}\nlandmass: {:.4} (internal {:.1})\nsquish: {:.2}\ndensity: {:.2}",
                    heightmap_noise.get_noise2d(pos.x, pos.z),
                    landmass_noise.get_noise2d(pos.x, pos.z),
                    landmass_noise.noise.get_noise(pos.x, pos.z),
                    squish_noise.get_noise2d(pos.x,pos.z),
                    density_noise.get_noise3d(pos.x, pos.y, pos.z)
                ),
                resources.0.clone(),
            );
        }
    }
}



fn draw_gizmos(mut gizmo: Gizmos, tf_query: Query<&GlobalTransform, With<DebugDrawTransform>>) {
    for tf in tf_query.iter() {
        gizmo.ray(tf.translation(), tf.forward(), Color::RED);
    }
}
