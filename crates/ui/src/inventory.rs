use bevy::{
    pbr::ExtendedMaterial,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
    ui::RelativeCursorPosition,
    window::PrimaryWindow,
};
use leafwing_input_manager::prelude::ActionState;

use engine::{
    actors::{LocalPlayer, LocalPlayerSpawnedEvent},
    controllers::{player_controller::CursorLocked, Action},
    debug::TextStyle,
    items::{block_item::BlockItem, inventory::Inventory, ItemIcon, ItemStack, MaxStackSize},
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
pub struct BlockPreview(Entity);

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
                update_mouse_display,
                player_scroll_inventory,
            )
                .in_set(LevelSystemSet::Main),
        )
        .init_resource::<MouseInventory>()
        .add_systems(OnEnter(UIState::Inventory), show_inventory)
        .add_systems(OnEnter(UIState::Default), hide_inventory::<false>)
        .add_systems(OnEnter(UIState::Hidden), hide_inventory::<true>)
        .add_systems(Startup, init);
    }
}

#[derive(Resource, Clone)]
pub(crate) struct InventoryResources {
    item_counts: TextStyle,
    slot_background: Handle<Image>,
    selection_image: Handle<Image>,
}

#[derive(Component)]
struct InventoryUI;

#[derive(Component, Clone)]
#[require(RelativeCursorPosition)]
struct InventoryUISlotBackground {
    inventory: Entity,
    slot: usize,
}

#[derive(Component, Clone, Copy)]
enum InventoryUISlot {
    Mouse,
    Entity { inventory: Entity, slot: usize },
}

impl InventoryUISlot {
    fn get_stack(
        self,
        mouse: &MouseInventory,
        inventory_query: &Query<&Inventory>,
    ) -> Option<ItemStack> {
        match self {
            InventoryUISlot::Mouse => mouse.selected.map(|m| m.stack),
            InventoryUISlot::Entity { inventory, slot } => inventory_query
                .get(inventory)
                .ok()
                .and_then(|inv| inv.get(slot))
                .and_then(|stack| {
                    //subtract off the items currently held in the mouse
                    match mouse.selected {
                        Some(selected) => {
                            let new_stack_size = stack.size.saturating_sub(selected.stack.size);
                            if inventory == selected.inventory
                                && selected.slot == slot
                                && selected.stack.id == stack.id
                            {
                                if new_stack_size > 0 {
                                    Some(ItemStack::new(stack.id, new_stack_size))
                                } else {
                                    None
                                }
                            } else {
                                Some(stack)
                            }
                        }
                        None => Some(stack),
                    }
                }),
        }
    }
}

#[derive(Component, Default, Clone, Copy)]
struct InventoryUISlotIcon(Option<Entity>);

#[derive(Component, Default, Clone, Copy)]
struct InventoryUISlotText(u32);

#[derive(Component)]
struct InventoryUISelector;

#[derive(Resource, Default)]
struct MouseInventory {
    selected: Option<MouseInventorySelected>,
}

impl MouseInventory {
    fn clear(&mut self) {
        self.selected = None;
    }
}

#[derive(Clone, Copy, Debug)]
struct MouseInventorySelected {
    slot: usize,
    stack: ItemStack,
    inventory: Entity,
    click_offset: Vec2,
}

#[derive(Component)]
#[require(Node)]
struct MouseInventoryVisual;

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
) {
    for LocalPlayerSpawnedEvent(id) in event_reader.read() {
        info!("inventory UI trying to spawn from LocalPlayerSpawned event");
        if let Ok(inv) = inventory_query.get(*id) {
            info!("spawning inventory UI with {} slots!", inv.len());
            for entity in inventory_ui_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
            spawn_inventory(&mut commands, *id, inv.len(), &resources);
            match state.get() {
                UIState::Hidden => commands.run_system_cached(hide_inventory::<true>),
                UIState::Default => commands.run_system_cached(hide_inventory::<false>),
                UIState::Inventory => commands.run_system_cached(show_inventory),
            };
            commands.run_system_cached(update_counts);
            commands.run_system_cached(update_icons);
            return;
        }
    }
}

fn show_inventory(
    mut inventory_query: Query<
        &mut Visibility,
        (With<InventoryUISlotBackground>, With<InventoryUI>),
    >,
    mut slot_query: Query<&mut Visibility, (With<InventoryUISlotBackground>, Without<InventoryUI>)>,
) {
    for mut vis in inventory_query.iter_mut() {
        info!("showing inventory");
        *vis.as_mut() = Visibility::Inherited;
    }
    //display all slots
    for mut slot in slot_query.iter_mut() {
        *slot.as_mut() = Visibility::Inherited;
    }
}

fn hide_inventory<const HIDE_HOTBAR: bool>(
    mut slot_query: Query<(&mut Visibility, &InventoryUISlotBackground), Without<InventoryUI>>,
    mut inventory_query: Query<
        &mut Visibility,
        (Without<InventoryUISlotBackground>, With<InventoryUI>),
    >,
) {
    for mut vis in inventory_query.iter_mut() {
        info!("hiding inventory");
        *vis.as_mut() = if HIDE_HOTBAR {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for (mut vis, slot) in slot_query.iter_mut() {
        //make sure hotbar slots are shown if needed
        if !HIDE_HOTBAR && slot.slot < HOTBAR_SLOTS {
            *vis.as_mut() = Visibility::Inherited;
        } else {
            //hide everything else (or hotbar slots too if HIDE_HOTBAR is true)
            *vis.as_mut() = Visibility::Hidden;
        }
    }
}

pub(crate) fn spawn_item_slot<'a>(
    mut ec: EntityCommands<'a>,
    background_node: Node,
    background_bundle: impl Bundle,
    children_bundle: impl Bundle + Clone,
    resources: &'_ InventoryResources,
) -> EntityCommands<'a> {
    let icon = (
        Node {
            width: Val::Px(SLOT_PX),
            height: Val::Px(SLOT_PX),
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode::default(),
        Visibility::Hidden,
        InventoryUISlotIcon::default(),
        PickingBehavior::IGNORE,
    );
    let count_parent = (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::FlexEnd,
            padding: UiRect::right(Val::Px(STACK_SIZE_LABEL_PADDING_PX)),
            ..default()
        },
        PickingBehavior::IGNORE,
    );
    let count = (
        Text::new("0".to_string()),
        TextLayout::new_with_justify(JustifyText::Right),
        resources.item_counts.clone(),
        Visibility::Hidden,
        InventoryUISlotText::default(),
    );
    ec.insert((
        background_node,
        background_bundle,
        ImageNode::new(resources.slot_background.clone()),
    ))
    .with_children(|slot_content| {
        //spawn the slot content - this is where the item images go
        slot_content.spawn((icon.clone(), children_bundle.clone()));
        //this is the stack size label
        //making a parent to anchor the label to the bottom right
        slot_content
            .spawn(count_parent.clone())
            .with_children(|label| {
                label.spawn((count.clone(), children_bundle.clone()));
            });
    });
    ec
}

pub(crate) fn default_slot_background() -> Node {
    Node {
        aspect_ratio: Some(1.0),
        margin: UiRect::all(Val::Px(1.0)),
        width: Val::Px(SLOT_PX),
        height: Val::Px(SLOT_PX),
        position_type: PositionType::Absolute,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn spawn_inventory(
    commands: &mut Commands,
    owner: Entity,
    slots: usize,
    resources: &InventoryResources,
) {
    let background = default_slot_background();
    // mouse inventory
    spawn_item_slot(
        commands.spawn(StateScoped(GameState::Game)),
        Node {
            position_type: PositionType::Absolute,
            ..background.clone()
        },
        (
            MouseInventoryVisual,
            Visibility::Inherited,
            PickingBehavior::IGNORE,
            MainCameraUIRoot,
            Name::new("Mouse inventory"),
            GlobalZIndex(1),
        ),
        InventoryUISlot::Mouse,
        resources,
    )
    .remove::<ImageNode>();
    // main inventory
    commands
        .spawn((
            StateScoped(GameState::Game),
            MainCameraUIRoot,
            PickingBehavior::IGNORE,
            Name::new("Inventory UI"),
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
                spawn_item_slot(
                    slot_background.spawn_empty(),
                    Node {
                        left: slot_coords.left,
                        right: slot_coords.right,
                        top: slot_coords.top,
                        bottom: slot_coords.bottom,
                        ..background.clone()
                    },
                    (InventoryUISlotBackground {
                        slot,
                        inventory: owner,
                    },),
                    InventoryUISlot::Entity {
                        slot,
                        inventory: owner,
                    },
                    resources,
                )
                .observe(slot_clicked);
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
                PickingBehavior::IGNORE,
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
    mut label_query: Query<(
        &mut Visibility,
        &mut Text,
        &InventoryUISlot,
        &InventoryUISlotText,
    )>,
    inventory_query: Query<&Inventory>,
    mouse: Res<MouseInventory>,
) {
    for (mut vis, mut text, ui_slot, prev_value) in label_query.iter_mut() {
        match ui_slot.get_stack(&mouse, &inventory_query) {
            Some(stack) => {
                *vis.as_mut() = Visibility::Inherited;
                if prev_value.0 != stack.size {
                    text.0 = stack.size.to_string();
                }
            }
            None => {
                *vis.as_mut() = Visibility::Hidden;
            }
        }
    }
}

fn update_icons(
    mut label_query: Query<(
        &mut Visibility,
        &mut ImageNode,
        &InventoryUISlot,
        &mut InventoryUISlotIcon,
    )>,
    mut images: ResMut<Assets<Image>>,
    inventory_query: Query<&Inventory>,
    icon_query: Query<&ItemIcon>,
    block_item_query: Query<&BlockItem>,
    block_mesh_query: Query<&BlockMesh>,
    block_preview_query: Query<&BlockPreview>,
    materials: Res<ChunkMaterial>,
    mouse: Res<MouseInventory>,
    mut commands: Commands,
) {
    fn clear_slot(commands: &mut Commands, icon: &mut InventoryUISlotIcon) {
        if let Some(stored_entity) = icon.0 {
            if let Some(ec) = commands.get_entity(stored_entity) {
                ec.despawn_recursive();
            }
            icon.0 = None;
        }
    }
    for (index, (mut vis, mut image, ui_slot, mut icon)) in label_query.iter_mut().enumerate() {
        let Some(stack) = ui_slot.get_stack(&mouse, &inventory_query) else {
            clear_slot(&mut commands, &mut icon);
            *vis.as_mut() = Visibility::Hidden;
            continue;
        };
        match icon_query.get(stack.id) {
            Ok(icon) => {
                *vis.as_mut() = Visibility::Inherited;
                image.image = icon.0.clone();
            }
            Err(_) => {
                //todo - cache these
                match block_item_query.get(stack.id) {
                    Ok(item) => {
                        if let Some(old_icon) = icon.0
                            && let Ok(old_preview) = block_preview_query.get(old_icon)
                            && old_preview.0 == item.0
                        {
                            //we already have a block preview for this block, no need to re-render
                            continue;
                        }
                        //despawn old block item
                        clear_slot(&mut commands, &mut icon);
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
                                    Vec3::new(index as f32 * 5.0, 0.0, 0.0) + PREVIEW_ORIGIN,
                                    item.0,
                                );
                                icon.0 = Some(preview_entity);
                                image.image = preview;
                                *vis.as_mut() = Visibility::Inherited;
                            }
                            None => *vis.as_mut() = Visibility::Hidden,
                        }
                    }
                    Err(_) => *vis.as_mut() = Visibility::Hidden,
                }
            }
        }
    }
}

fn spawn_block_preview(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    mesh: Handle<Mesh>,
    material: Handle<ExtendedMaterial<StandardMaterial, TextureArrayExtension>>,
    position: Vec3,
    block: Entity,
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
            BlockPreview(block),
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

fn update_mouse_display(
    mouse: Res<MouseInventory>,
    mut parent_query: Query<&mut Node, With<MouseInventoryVisual>>,
    window: Single<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
) {
    let Some(position) = window.cursor_position() else {
        return;
    };
    let Some(mouse) = mouse.selected else {
        return;
    };
    for mut node in parent_query.iter_mut() {
        node.left = Val::Px((position.x - mouse.click_offset.x) / scale.0);
        node.top = Val::Px((position.y - mouse.click_offset.y) / scale.0);
    }
}

fn slot_clicked(
    trigger: Trigger<Pointer<Click>>,
    slot_query: Query<(
        &InventoryUISlotBackground,
        &RelativeCursorPosition,
        &ComputedNode,
    )>,
    mut inventory_query: Query<&mut Inventory>,
    mut mouse: ResMut<MouseInventory>,
    stack_query: Query<&MaxStackSize>,
    focused: Res<CursorLocked>,
) {
    if focused.0 {
        return;
    }
    let Ok((target_slot, cursor_offset, slot_node)) = slot_query.get(trigger.entity()) else {
        return;
    };
    let mut moved_out_of_mouse = 0;
    match mouse.selected.as_mut() {
        Some(selected) => {
            // mouse has items, so we need to try to stack against the target slot
            // left click = stack as many as possible (or swap items if not possible)
            // right click = stack one
            // middle click = stack half?
            // feels weird since middle/right click are reversed, but feel like it's more useful to pick up half a stack and drop one item at a time.

            // 2 cases - either swapping within one inventory, or moving between two inventories:
            if selected.inventory == target_slot.inventory {
                // only 1 inventory involved
                let Ok(mut inventory) = inventory_query.get_mut(selected.inventory) else {
                    //invalid inventory, clear the mouse
                    info!("invalid inventory");
                    mouse.clear();
                    return;
                };
                if !matches!(inventory.get(selected.slot), Some(x) if x.id == selected.stack.id) {
                    //item changed out from under us (likely due to poor game design), clear the mouse to avoid confusing behavior
                    mouse.clear();
                    return;
                }
                let target_item = inventory.get(target_slot.slot);
                let desired_move_count = match trigger.button {
                    PointerButton::Primary => u32::MAX,
                    PointerButton::Secondary => 1,
                    PointerButton::Middle => selected.stack.size.div_ceil(2),
                }
                .min(selected.stack.size);
                moved_out_of_mouse = if selected.slot != target_slot.slot {
                    inventory.move_items(
                        selected.slot,
                        target_slot.slot,
                        desired_move_count,
                        &stack_query,
                    )
                } else {
                    // if we're moving back to the spot we picked up from, we only have to update the mouse
                    // the inventory will automatically update the icon in that spot
                    selected.stack.size.min(desired_move_count)
                };

                if moved_out_of_mouse == 0 && trigger.button == PointerButton::Primary {
                    //left click which couldn't stack, swap items
                    inventory.swap_slots(selected.slot, target_slot.slot);
                    if let Some(target) = target_item {
                        selected.stack = target;
                        selected.click_offset =
                            cursor_offset.normalized.unwrap_or_default() * slot_node.size();
                    } else {
                        moved_out_of_mouse = u32::MAX;
                    }
                }
            } else {
                // swapping between two inventories
                let Ok([_target_inventory, _mouse_inventory]) =
                    inventory_query.get_many_mut([target_slot.inventory, selected.inventory])
                else {
                    //invalid inventory, clear the mouse
                    mouse.clear();
                    return;
                };
                todo!("Inventory UI needs to support multiple inventories");
            }
        }
        None => {
            let Ok(target_inventory) = inventory_query.get(target_slot.inventory) else {
                //invalid inventory, clear the mouse
                mouse.clear();
                return;
            };
            if let Some(stack) = target_inventory.get(target_slot.slot) {
                //mouse is empty, pick up whole stack if left click, half if right click
                let new_size = match trigger.button {
                    PointerButton::Primary => stack.size,
                    PointerButton::Secondary => stack.size.div_ceil(2),
                    PointerButton::Middle => 1,
                };

                mouse.selected = if new_size > 0 {
                    Some(MouseInventorySelected {
                        slot: target_slot.slot,
                        stack: ItemStack::new(stack.id, new_size),
                        inventory: target_slot.inventory,
                        click_offset: cursor_offset.normalized.unwrap_or_default()
                            * slot_node.size(),
                    })
                } else {
                    None
                };
            }
        }
    }

    if moved_out_of_mouse > 0 {
        mouse.selected = match mouse.selected {
            Some(selected) => {
                let new_size = selected.stack.size.saturating_sub(moved_out_of_mouse);
                if new_size == 0 {
                    None
                } else {
                    Some(MouseInventorySelected {
                        stack: ItemStack::new(selected.stack.id, new_size),
                        ..selected
                    })
                }
            }
            None => None,
        };
    }
}

fn player_scroll_inventory(
    mut query: Query<&mut Inventory, With<LocalPlayer>>,
    focused: Res<CursorLocked>,
    action: Res<ActionState<Action>>,
) {
    if !focused.0 {
        return;
    }
    const SCROLL_SENSITIVITY: f32 = 0.05;
    if let Ok(mut inv) = query.get_single_mut() {
        let delta = action.value(&Action::Scroll);
        let slot_diff = if delta > SCROLL_SENSITIVITY {
            -1
        } else if delta < -SCROLL_SENSITIVITY {
            1
        } else {
            0
        };
        let curr_slot = inv.selected_slot();
        let new_slot = (curr_slot as i32 + slot_diff).rem_euclid(HOTBAR_SLOTS as i32);
        inv.select_slot(new_slot);
    }
}
