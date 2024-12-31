use bevy::{
    ecs::system::SystemId,
    pbr::ExtendedMaterial,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
};
use leafwing_input_manager::prelude::ActionState;

use engine::{
    actors::{LocalPlayer, LocalPlayerSpawnedEvent},
    controllers::Action,
    items::{
        block_item::BlockItem, inventory::Inventory, DropItemEvent, ItemIcon, PickupItemEvent,
    },
    mesher::{extended_materials::TextureArrayExtension, ChunkMaterial},
    world::{BlockMesh, LevelSystemSet},
    GameState,
};

use crate::MainCameraUIRoot;

use super::{state::UIState, styles::get_small_text_style};

pub const SLOTS_PER_ROW: usize = 10;
pub const HOTBAR_SLOTS: usize = SLOTS_PER_ROW;
pub const BACKGROUND_COLOR: Color = Color::srgba(0.15, 0.15, 0.15, 0.25);

const MARGIN_PX: f32 = 1.0;
const SLOT_PX: f32 = 32.0;
const SELECTOR_PADDING_PX: f32 = 1.0;
const STACK_SIZE_LABEL_PADDING_PX: f32 = 3.0;

pub const BLOCK_PREVIEW_LAYER: RenderLayers = RenderLayers::layer(1);

pub struct InventoryPlugin;

#[derive(Component)]
pub struct BlockPreview;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                toggle_inventory,
                place_inventory_selector,
                spawn_inventory_system,
                update_counts,
                update_icons,
            )
                .in_set(LevelSystemSet::Main),
        )
        .add_systems(OnEnter(UIState::Inventory), show_inventory)
        .add_systems(OnEnter(UIState::Default), hide_inventory::<false>)
        .add_systems(OnEnter(UIState::Hidden), hide_inventory::<true>)
        .add_systems(Startup, init);

        let show_inventory_id = app.world_mut().register_system(show_inventory);
        let hide_inventory_id = app.world_mut().register_system(hide_inventory::<false>);
        let hide_inventory_and_hotbar_id = app.world_mut().register_system(hide_inventory::<true>);
        let update_counts_id = app.world_mut().register_system(update_counts);
        let update_icons_id = app.world_mut().register_system(update_icons);
        app.insert_resource(InventorySystemIds {
            show_inventory: show_inventory_id,
            hide_inventory: hide_inventory_id,
            hide_inventory_and_hotbar: hide_inventory_and_hotbar_id,
            update_counts: update_counts_id,
            update_icons: update_icons_id,
        });
    }
}

#[derive(Resource)]
struct InventoryResources {
    item_counts: (TextColor, TextFont),
    slot_background: Handle<Image>,
    selection_image: Handle<Image>,
}

#[derive(Resource)]
struct InventorySystemIds {
    show_inventory: SystemId,
    hide_inventory_and_hotbar: SystemId,
    hide_inventory: SystemId,
    update_icons: SystemId,
    update_counts: SystemId,
}

#[derive(Component)]
struct InventoryUI;

#[derive(Component)]
struct InventoryUISlotBackground(usize);

#[derive(Component)]
//slot num, entity stored in slot
struct InventoryUISlot(usize, Option<Entity>);

#[derive(Component)]
struct InventoryUISelector;

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(InventoryResources {
        item_counts: get_small_text_style(&assets),
        slot_background: assets.load("textures/inventory_tile.png"),
        selection_image: assets.load("textures/selection.png"),
    });
}

fn toggle_inventory(
    mut next_state: ResMut<NextState<UIState>>,
    state: Res<State<UIState>>,
    action: Res<ActionState<Action>>,
) {
    if action.just_pressed(&Action::ToggleInventory) {
        next_state.set(if *state.get() == UIState::Inventory {
            UIState::Default
        } else {
            UIState::Inventory
        });
    }
}

fn spawn_inventory_system(
    mut event_reader: EventReader<LocalPlayerSpawnedEvent>,
    inventory_ui_query: Query<Entity, With<InventoryUI>>,
    inventory_query: Query<&Inventory, With<LocalPlayer>>,
    mut commands: Commands,
    resources: Res<InventoryResources>,
    state: Res<State<UIState>>,
    system_ids: Res<InventorySystemIds>,
) {
    for LocalPlayerSpawnedEvent(id) in event_reader.read() {
        info!("inventory UI trying to spawn from LocalPlayerSpawned event");
        if let Ok(inv) = inventory_query.get(*id) {
            info!("spawning inventory UI with {} slots!", inv.len());
            for entity in inventory_ui_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
            spawn_inventory(&mut commands, inv.len(), &resources);
            match state.get() {
                UIState::Hidden => commands.run_system(system_ids.hide_inventory_and_hotbar),
                UIState::Default => commands.run_system(system_ids.hide_inventory),
                UIState::Inventory => commands.run_system(system_ids.show_inventory),
            };
            commands.run_system(system_ids.update_counts);
            commands.run_system(system_ids.update_icons);
            return;
        }
    }
}

fn show_inventory(
    mut inventory_query: Query<(&InventoryUI, &mut Visibility), Without<InventoryUISlotBackground>>,
    mut slot_query: Query<&mut Visibility, With<InventoryUISlotBackground>>,
) {
    if let Ok((_, mut vis)) = inventory_query.get_single_mut() {
        *vis.as_mut() = Visibility::Inherited;
        //display all slots
        for mut slot in slot_query.iter_mut() {
            *slot.as_mut() = Visibility::Inherited;
        }
    }
}

fn hide_inventory<const HIDE_HOTBAR: bool>(
    mut slot_query: Query<(&mut Visibility, &InventoryUISlotBackground), Without<InventoryUI>>,
    mut inventory_query: Query<(&InventoryUI, &mut Visibility), Without<InventoryUISlotBackground>>,
) {
    if let Ok((_, mut vis)) = inventory_query.get_single_mut() {
        *vis.as_mut() = if HIDE_HOTBAR {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };

        for (mut vis, slot) in slot_query.iter_mut() {
            //make sure hotbar slots are shown if needed
            if !HIDE_HOTBAR && slot.0 < HOTBAR_SLOTS {
                *vis.as_mut() = Visibility::Inherited;
            } else {
                //hide everything else (or hotbar slots too if HIDE_HOTBAR is true)
                *vis.as_mut() = Visibility::Hidden;
            }
        }
    }
}

fn spawn_inventory(commands: &mut Commands, slots: usize, resources: &InventoryResources) {
    commands
        .spawn((
            StateScoped(GameState::Game),
            MainCameraUIRoot,
            InventoryUI,
            Node {
                width: Val::Px(400.0),
                height: Val::Px(200.0),
                align_items: AlignItems::FlexStart,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .with_children(|slot_background| {
            //spawn rows - round up number of rows
            for slot in 0..slots {
                let slot_coords = get_slot_coords(slot, 0.0);
                slot_background
                    .spawn((
                        Node {
                            aspect_ratio: Some(1.0),
                            margin: UiRect::all(Val::Px(1.0)),
                            width: Val::Px(SLOT_PX),
                            height: Val::Px(SLOT_PX),
                            left: slot_coords.left,
                            right: slot_coords.right,
                            bottom: slot_coords.bottom,
                            top: slot_coords.top,
                            position_type: PositionType::Absolute,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        ImageNode::new(resources.slot_background.clone()),
                        InventoryUISlotBackground(slot),
                    ))
                    .with_children(|slot_content| {
                        //spawn the slot content - this is where the item images go
                        slot_content.spawn((
                            Node {
                                width: Val::Px(SLOT_PX),
                                height: Val::Px(SLOT_PX),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            ImageNode::default(),
                            Visibility::Hidden,
                            InventoryUISlot(slot, None),
                        ));
                        //this is the stack size label
                        //making a parent to anchor the label to the bottom right
                        slot_content
                            .spawn(Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                position_type: PositionType::Absolute,
                                justify_content: JustifyContent::FlexEnd,
                                align_items: AlignItems::FlexEnd,
                                padding: UiRect::right(Val::Px(STACK_SIZE_LABEL_PADDING_PX)),
                                ..default()
                            })
                            .with_children(|label| {
                                label.spawn((
                                    Text::new("0".to_string()),
                                    TextLayout::new_with_justify(JustifyText::Right),
                                    resources.item_counts.clone(),
                                    Visibility::Hidden,
                                    InventoryUISlot(slot, None),
                                ));
                            });
                    });
            }
        })
        .with_children(|selector| {
            selector.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(SLOT_PX),
                    height: Val::Px(SLOT_PX),
                    ..default()
                },
                ImageNode::new(resources.selection_image.clone()),
                InventoryUISelector,
            ));
        });
    info!("inventory spawned!")
}

fn get_slot_coords(slot: usize, offset_px: f32) -> UiRect {
    let row = slot / SLOTS_PER_ROW;
    let col = slot % SLOTS_PER_ROW;
    const STRIDE: f32 = MARGIN_PX + SLOT_PX;
    UiRect {
        left: Val::Px(offset_px + MARGIN_PX + col as f32 * STRIDE),
        top: Val::Px(offset_px + MARGIN_PX + row as f32 * STRIDE),
        ..default()
    }
}

fn place_inventory_selector(
    mut selector_query: Query<&mut Node, With<InventoryUISelector>>,
    inventory_query: Query<&Inventory, With<LocalPlayer>>,
) {
    if let Ok(inv) = inventory_query.get_single() {
        if let Ok(mut style) = selector_query.get_single_mut() {
            let coords = get_slot_coords(inv.selected_slot(), SELECTOR_PADDING_PX);
            style.left = coords.left;
            style.right = coords.right;
            style.top = coords.top;
            style.bottom = coords.bottom;
        }
    }
}

fn update_counts(
    mut label_query: Query<(&mut Visibility, &mut Text, &InventoryUISlot)>,
    pickup_reader: EventReader<PickupItemEvent>,
    drop_reader: EventReader<DropItemEvent>,
    inventory_query: Query<&Inventory, (With<LocalPlayer>, Changed<Inventory>)>,
) {
    if pickup_reader.is_empty() && drop_reader.is_empty() {
        return;
    }
    if let Ok(inv) = inventory_query.get_single() {
        for (mut vis, mut text, ui_slot) in label_query.iter_mut() {
            match inv.get(ui_slot.0) {
                Some(stack) => {
                    *vis.as_mut() = Visibility::Inherited;
                    text.0 = stack.size.to_string();
                }
                None => {
                    *vis.as_mut() = Visibility::Hidden;
                }
            }
        }
    }
}

fn update_icons(
    mut label_query: Query<(&mut Visibility, &mut ImageNode, &mut InventoryUISlot)>,
    mut images: ResMut<Assets<Image>>,
    pickup_reader: EventReader<PickupItemEvent>,
    drop_reader: EventReader<DropItemEvent>,
    inventory_query: Query<&Inventory, (With<LocalPlayer>, Changed<Inventory>)>,
    icon_query: Query<&ItemIcon>,
    block_item_query: Query<&BlockItem>,
    block_mesh_query: Query<&BlockMesh>,
    materials: Res<ChunkMaterial>,
    mut commands: Commands,
) {
    if pickup_reader.is_empty() && drop_reader.is_empty() {
        return;
    }
    if let Ok(inv) = inventory_query.get_single() {
        for (index, (mut vis, mut image, mut ui_slot)) in label_query.iter_mut().enumerate() {
            if let Some(stored_entity) = ui_slot.1 {
                if let Some(ec) = commands.get_entity(stored_entity) {
                    ec.despawn_recursive();
                }
                ui_slot.1 = None;
            }
            match inv.get(ui_slot.0) {
                Some(stack) => match icon_query.get(stack.id) {
                    Ok(icon) => {
                        *vis.as_mut() = Visibility::Inherited;
                        image.image = icon.0.clone();
                    }
                    Err(_) => {
                        match block_item_query.get(stack.id) {
                            Ok(item) => {
                                //render block item
                                //todo - despawn if dynamic
                                match block_mesh_query
                                    .get(item.0)
                                    .ok()
                                    .and_then(|block_mesh| block_mesh.single_mesh.as_ref())
                                {
                                    Some(mesh) => {
                                        //spawn these entities super far out because lighting affects all layers
                                        //this way they (probably) won't end up in the shadow of some terrain
                                        const PREVIEW_ORIGIN: Vec3 =
                                            Vec3::new(1_000_000.0, 1_000_000.0, 1_000_000.0);
                                        let (preview_entity, preview) = spawn_block_preview(
                                            &mut commands,
                                            &mut images,
                                            mesh.clone(),
                                            materials.opaque_material.clone().unwrap(),
                                            Vec3::new(index as f32 * 5.0, 0.0, 0.0)
                                                + PREVIEW_ORIGIN,
                                        );
                                        ui_slot.1 = Some(preview_entity);
                                        image.image = preview;
                                        *vis.as_mut() = Visibility::Inherited;
                                    }
                                    None => *vis.as_mut() = Visibility::Hidden,
                                }
                            }
                            Err(_) => *vis.as_mut() = Visibility::Hidden,
                        }
                    }
                },
                None => {
                    *vis.as_mut() = Visibility::Hidden;
                }
            }
        }
    }
}

fn spawn_block_preview(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    mesh: Handle<Mesh>,
    material: Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>,
    position: Vec3,
) -> (Entity, Handle<Image>) {
    // This code for rendering to a texture is taken from one of the Bevy examples,
    // https://github.com/bevyengine/bevy/blob/main/examples/3d/render_to_texture.rs
    let size = Extent3d {
        width: SLOT_PX as u32,
        height: SLOT_PX as u32,
        ..default()
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // Fill image.data with zeros
    image.resize(size);

    let image_handle = images.add(image);

    #[allow(state_scoped_entities)]
    let entity = commands
        .spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(position),
            BlockPreview,
            BLOCK_PREVIEW_LAYER,
        ))
        .with_children(|children| {
            //spawn camera as a child so that we can just despawn_recursive() the one entity
            const CAMERA_OFFSET: Vec3 = Vec3::new(1.0, 1.0, 1.0);
            children
                .spawn((
                    Camera3d::default(),
                    Camera {
                        // render before the main camera
                        order: -1,
                        target: RenderTarget::Image(image_handle.clone()),
                        clear_color: ClearColorConfig::Custom(Color::NONE),
                        ..default()
                    },
                    Projection::Orthographic(OrthographicProjection {
                        scale: 2.0 / SLOT_PX, // smaller numbers here make the block look bigger
                        ..OrthographicProjection::default_3d()
                    }),
                    Transform::from_translation(CAMERA_OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
                ))
                // only render the block previews
                .insert(BLOCK_PREVIEW_LAYER);
        })
        .id();

    (entity, image_handle)
}
