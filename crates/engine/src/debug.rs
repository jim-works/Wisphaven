use ahash::HashMap;
use bevy::{prelude::*, utils::HashSet};
use leafwing_input_manager::action_state::ActionState;

use crate::{
    actors::LocalPlayer,
    controllers::Action,
    physics::{collision::Aabb, PhysicsSystemSet},
    world::{chunk::ChunkCoord, BlockCoord, BlockPhysics},
    worldgen::UsedShaperResources,
};

pub struct DebugUIPlugin;

impl Plugin for DebugUIPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<DebugUIState>()
            .init_state::<DebugUIDetailState>()
            .insert_resource(DebugBlockHitboxes::default())
            .insert_resource(FixedUpdateBlockGizmos::default())
            .add_systems(Startup, init)
            .add_systems(OnEnter(DebugUIState::Shown), spawn_debug)
            .add_systems(OnEnter(DebugUIState::Hidden), despawn_debug)
            .add_systems(Update, toggle_debug)
            .add_systems(
                Update,
                (
                    update_coords,
                    update_chunk_coords,
                    update_noises,
                    update_gizmos,
                    toggle_gizmo_depth,
                )
                    .run_if(in_state(DebugUIState::Shown)),
            )
            .add_systems(
                FixedUpdate,
                clear_fixed_update_gizmos.before(PhysicsSystemSet::Main),
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

#[derive(Component, Default)]
pub struct DebugDrawTransform;

#[derive(Resource, Default)]
pub struct DebugBlockHitboxes {
    pub blocks: HashMap<BlockCoord, Option<BlockPhysics>>,
    pub hit_blocks: HashSet<BlockCoord>,
}

#[derive(Resource, Default)]
pub struct FixedUpdateBlockGizmos {
    pub blocks: HashSet<BlockCoord>,
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub enum DebugUIState {
    #[default]
    Hidden,
    Shown,
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub enum DebugUIDetailState {
    #[default]
    Minimal,
    Most,
}

fn init(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(DebugResources(get_text_style(&assets)));
}

fn get_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
        font_size: 32.0,
        color: Color::WHITE,
    }
}

pub fn toggle_debug(
    mut next_state: ResMut<NextState<DebugUIState>>,
    curr_state: Res<State<DebugUIState>>,
    mut detail_next_state: ResMut<NextState<DebugUIDetailState>>,
    detail_curr_state: Res<State<DebugUIDetailState>>,
    action: Res<ActionState<Action>>,
) {
    if action.just_pressed(&Action::ToggleDebugUIHidden) {
        match curr_state.get() {
            DebugUIState::Hidden => next_state.set(DebugUIState::Shown),
            _ => next_state.set(DebugUIState::Hidden),
        }
    }
    if action.just_pressed(&Action::ToggleDebugUIDetail) {
        let next = match detail_curr_state.get() {
            DebugUIDetailState::Minimal => DebugUIDetailState::Most,
            _ => DebugUIDetailState::Minimal,
        };
        info!("Debug detail set to {:?}", next);
        detail_next_state.set(next);
    }
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
                            justify: JustifyText::Left,
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
                            justify: JustifyText::Left,
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
                            justify: JustifyText::Left,
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

fn clear_fixed_update_gizmos(mut fixed_update_blocks: ResMut<FixedUpdateBlockGizmos>) {
    fixed_update_blocks.blocks.clear();
}

fn toggle_gizmo_depth(
    mut gizmo_config: ResMut<GizmoConfigStore>,
    action: Res<ActionState<Action>>,
) {
    let (config, _) = gizmo_config.config_mut::<DefaultGizmoConfigGroup>();
    if action.just_pressed(&Action::ToggleGizmoOverlap) {
        config.depth_bias = if config.depth_bias == 0.0 { -1.0 } else { 0.0 };
    }
}

fn update_gizmos(
    mut gizmo: Gizmos,
    tf_query: Query<&GlobalTransform, With<DebugDrawTransform>>,
    collider_query: Query<(&GlobalTransform, &Aabb)>,
    blocks: Res<DebugBlockHitboxes>,
    fixed_update_blocks: Res<FixedUpdateBlockGizmos>,
    detail: Res<State<DebugUIDetailState>>,
) {
    for tf in tf_query.iter() {
        gizmo.ray(tf.translation(), *tf.forward(), Color::srgb(1., 0., 0.));
    }
    for (gtf, collider) in collider_query.iter() {
        let cuboid_tf = Transform::from_translation(collider.world_center(gtf.translation()))
            .with_scale(collider.size);
        gizmo.cuboid(cuboid_tf, Color::srgb(0., 0., 1.))
    }
    for coord in fixed_update_blocks.blocks.iter() {
        let cuboid_tf = Transform::from_translation(coord.center()).with_scale(Vec3::ONE);
        gizmo.cuboid(cuboid_tf, Color::srgb(0.7, 0.7, 0.1))
    }
    for (coord, physics) in blocks.blocks.iter() {
        let collider_opt = physics.clone().and_then(|p| Aabb::from_block(&p));
        if let Some(collider) = collider_opt {
            let cuboid_tf = Transform::from_translation(collider.world_center(coord.to_vec3()))
                .with_scale(collider.size);
            if *detail.get() == DebugUIDetailState::Most {
                gizmo.cuboid(
                    cuboid_tf,
                    if blocks.hit_blocks.contains(coord) {
                        Color::srgb(0.8, 0.2, 0.)
                    } else {
                        Color::srgb(0., 1., 0.)
                    },
                )
            } else if blocks.hit_blocks.contains(coord) {
                gizmo.cuboid(cuboid_tf, Color::srgb(0.8, 0.2, 0.));
            }
        }
    }
}
