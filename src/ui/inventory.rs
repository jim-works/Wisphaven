use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    actors::{LocalPlayer, LocalPlayerSpawnedEvent},
    controllers::Action,
    items::{inventory::Inventory, ItemIcon},
    world::LevelSystemSet,
};

use super::{state::UIState, styles::get_small_text_style};

pub const SLOTS_PER_ROW: usize = 10;
pub const HOTBAR_SLOTS: usize = SLOTS_PER_ROW;
pub const BACKGROUND_COLOR: BackgroundColor = BackgroundColor(Color::Rgba {
    red: 0.15,
    green: 0.15,
    blue: 0.15,
    alpha: 0.25,
});

const MARGIN_PX: f32 = 1.0;
const SLOT_PX: f32 = 32.0;
const SELECTOR_PADDING_PX: f32 = 1.0;
const STACK_SIZE_LABEL_PADDING_PX: f32 = 3.0;

pub struct InventoryPlugin;

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
    }
}

#[derive(Resource)]
struct InventoryResources {
    item_counts: TextStyle,
    slot_background: Handle<Image>,
    selection_image: Handle<Image>,
}

#[derive(Component)]
struct InventoryUI;

#[derive(Component)]
struct InventoryUISlotBackground(usize);

#[derive(Component)]
struct InventoryUISlot(usize);

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
    query: Query<&ActionState<Action>, With<LocalPlayer>>,
) {
    if let Ok(action) = query.get_single() {
        if action.just_pressed(Action::ToggleInventory) {
            next_state.set(if *state.get() == UIState::Inventory {
                UIState::Default
            } else {
                UIState::Inventory
            });
        }
    }
}

fn spawn_inventory_system(
    mut event_reader: EventReader<LocalPlayerSpawnedEvent>,
    inventory_ui_query: Query<(&InventoryUI, &mut Visibility)>,
    inventory_query: Query<&Inventory, With<LocalPlayer>>,
    mut commands: Commands,
    resources: Res<InventoryResources>,
) {
    if !inventory_ui_query.is_empty() {
        return;
    }
    for LocalPlayerSpawnedEvent(id) in event_reader.iter() {
        if let Ok(inv) = inventory_query.get(*id) {
            spawn_inventory(&mut commands, inv.len(), &resources);
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
            InventoryUI,
            NodeBundle {
                style: Style {
                    width: Val::Px(400.0), 
                    height: Val::Px(200.0),
                    align_items: AlignItems::FlexStart,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|slot_background| {
            //spawn rows - round up number of rows
            for slot in 0..slots {
                let slot_coords = get_slot_coords(slot, 0.0);
                slot_background
                    .spawn((
                        ImageBundle {
                            style: Style {
                                aspect_ratio: Some(1.0),
                                margin: UiRect::all(Val::Px(1.0)),
                                width: Val::Px(32.0),
                                height: Val::Px(32.0),
                                left: slot_coords.left,
                                right: slot_coords.right,
                                bottom: slot_coords.bottom,
                                top: slot_coords.top,
                                position_type: PositionType::Absolute,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            image: UiImage::new(resources.slot_background.clone()),
                            ..default()
                        },
                        InventoryUISlotBackground(slot),
                    ))
                    .with_children(|slot_content| {
                        //spawn the slot content - this is where the item images go
                        slot_content.spawn((
                            ImageBundle {
                                style: Style {
                                    width: Val::Px(32.0),
                                    height: Val::Px(32.0),
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                                visibility: Visibility::Hidden,
                                ..default()
                            },
                            InventoryUISlot(slot),
                        ));
                        //this is the stack size label
                        //making a parent to anchor the label to the bottom right
                        slot_content
                            .spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    justify_content: JustifyContent::FlexEnd,
                                    align_items: AlignItems::FlexEnd,
                                    padding: UiRect::right(Val::Px(STACK_SIZE_LABEL_PADDING_PX)),
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|label| {
                                label.spawn((
                                    TextBundle {
                                        text: Text {
                                            sections: vec![TextSection::new(
                                                "0",
                                                resources.item_counts.clone(),
                                            )],
                                            alignment: TextAlignment::Right,
                                            ..default()
                                        },
                                        visibility: Visibility::Hidden,
                                        ..default()
                                    },
                                    InventoryUISlot(slot),
                                ));
                            });
                    });
            }
        })
        .with_children(|selector| {
            selector.spawn((
                ImageBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Px(32.0),
                        height: Val::Px(32.0),
                        ..default()
                    },
                    image: UiImage::new(resources.selection_image.clone()),
                    ..default()
                },
                InventoryUISelector,
            ));
        });
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
    mut selector_query: Query<&mut Style, With<InventoryUISelector>>,
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
    inventory_query: Query<&Inventory, (With<LocalPlayer>, Changed<Inventory>)>,
) {
    if let Ok(inv) = inventory_query.get_single() {
        for (mut vis, mut text, ui_slot) in label_query.iter_mut() {
            match &inv[ui_slot.0] {
                Some(stack) => {
                    *vis.as_mut() = Visibility::Inherited;
                    text.sections[0].value = stack.size.to_string();
                }
                None => {
                    *vis.as_mut() = Visibility::Hidden;
                }
            }
        }
    }
}

fn update_icons(
    mut label_query: Query<(&mut Visibility, &mut UiImage, &InventoryUISlot)>,
    inventory_query: Query<&Inventory, (With<LocalPlayer>, Changed<Inventory>)>,
    icon_query: Query<&ItemIcon>,
) {
    if let Ok(inv) = inventory_query.get_single() {
        for (mut vis, mut image, ui_slot) in label_query.iter_mut() {
            match &inv[ui_slot.0] {
                Some(stack) => match icon_query.get(stack.id) {
                    Ok(icon) => {
                        *vis.as_mut() = Visibility::Inherited;
                        image.texture = icon.0.clone();
                    }
                    Err(_) => {
                        *vis.as_mut() = Visibility::Hidden;
                    }
                },
                None => {
                    *vis.as_mut() = Visibility::Hidden;
                }
            }
        }
    }
}
