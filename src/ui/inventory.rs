use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    actors::LocalPlayer, controllers::Action, items::inventory::Inventory, world::LevelSystemSet,
};

use super::{state::UIState, styles::get_small_text_style};

pub const SLOTS_PER_ROW: usize = 10;
pub const BACKGROUND_COLOR: BackgroundColor = BackgroundColor(Color::Rgba {
    red: 0.15,
    green: 0.15,
    blue: 0.15,
    alpha: 0.25,
});

const ROW_MARGIN_PX: f32 = 1.0;
const SLOT_MARGIN_PX: f32 = 1.0;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(toggle_inventory.in_set(LevelSystemSet::Main))
            .add_system(show_inventory.in_schedule(OnEnter(UIState::Inventory)))
            .add_system(hide_inventory.in_schedule(OnExit(UIState::Inventory)))
            .add_startup_system(init);
    }
}

#[derive(Resource)]
struct InventoryResources{
    item_counts: TextStyle,
    slot_background: Handle<Image>,
    selection_image: Handle<Image>,
}

#[derive(Component)]
struct InventoryUI;

#[derive(Component)]
struct InventoryUISlot(usize);

#[derive(Component)]
struct InventoryUIRow(usize);

#[derive(Component)]
struct InventoryUISelector(usize);

fn init(assets: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(InventoryResources {
        item_counts: get_small_text_style(&assets),
        slot_background: assets.load("textures/inventory_tile.png"),
        selection_image: assets.load("textures/selection.png")
    });
}

fn toggle_inventory(
    mut next_state: ResMut<NextState<UIState>>,
    state: Res<State<UIState>>,
    query: Query<&ActionState<Action>, With<LocalPlayer>>,
) {
    if let Ok(action) = query.get_single() {
        if action.just_pressed(Action::ToggleInventory) {
            next_state.set(if state.0 == UIState::Inventory {
                UIState::Default
            } else {
                UIState::Inventory
            });
        }
    }
}

fn show_inventory(
    query: Query<&Inventory, With<LocalPlayer>>,
    mut inventory_query: Query<&mut Visibility, With<InventoryUI>>,
    mut commands: Commands,
    resources: Res<InventoryResources>
) {
    if let Ok(mut inv_ui) = inventory_query.get_single_mut() {
        *inv_ui.as_mut() = Visibility::Inherited;
    } else if let Ok(inv) = query.get_single() {
        //spawn inventory since one doesn't exist
        spawn_inventory(&mut commands, inv.len(), resources.as_ref());
    }
}

fn hide_inventory(mut inventory_query: Query<&mut Visibility, With<InventoryUI>>) {
    if let Ok(mut inv_ui) = inventory_query.get_single_mut() {
        *inv_ui.as_mut() = Visibility::Hidden;
    }
}

fn spawn_inventory(commands: &mut Commands, slots: usize, resources: &InventoryResources) {
    commands
        .spawn((
            InventoryUI,
            NodeBundle {
                style: Style {
                    size: Size::new(Val::Px(400.0), Val::Px(200.0)),
                    align_items: AlignItems::FlexStart,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|rows| {
            //spawn rows - round up number of rows
            for row in 0..(slots+SLOTS_PER_ROW-1)/SLOTS_PER_ROW {
                rows.spawn((
                    NodeBundle {
                        style: Style {
                            margin: UiRect::all(Val::Px(ROW_MARGIN_PX)),
                            size: Size::new(Val::Percent(100.0), Val::Px(32.0)),
                            justify_content: JustifyContent::FlexStart,
                            ..default()
                        },
                        background_color: BACKGROUND_COLOR,
                        ..default()
                    },
                    InventoryUIRow(row),
                ))
                .with_children(|slot_background| {
                    //spawn slot backgrounds
                    for col in 0..SLOTS_PER_ROW {
                        let slot_idx = row * SLOTS_PER_ROW + col;
                        if slot_idx < slots {
                            slot_background
                                .spawn(ImageBundle {
                                    style: Style {
                                        aspect_ratio: Some(1.0),
                                        margin: UiRect::all(Val::Px(1.0)),
                                        size: Size::new(Val::Px(32.0), Val::Px(32.0)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    image: UiImage::new(resources.slot_background.clone()),
                                    ..default()
                                })
                                .with_children(|slot_content| {
                                    //spawn the slot content - this is where the item images go
                                    slot_content.spawn((
                                        ImageBundle {
                                            style: Style {
                                                size: Size::new(
                                                    Val::Px(32.0),
                                                    Val::Px(32.0),
                                                ),
                                                ..default()
                                            },
                                            visibility: Visibility::Hidden,
                                            ..default()
                                        },
                                        InventoryUISlot(slot_idx),
                                    ));
                                });
                        }
                    }
                });
            }
        })
        .with_children(|selector| {
            selector.spawn((ImageBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    size: Size::new(Val::Px(32.0), Val::Px(32.0)),
                    ..default()
                },
                image: UiImage::new(resources.selection_image.clone()),
                ..default()
            }, InventoryUISelector(0)));
        });
}

fn get_slot_coords(slot: usize) -> UiRect {
    let row = slot/SLOTS_PER_ROW;
    let col = slot%SLOTS_PER_ROW;
    let 
}

