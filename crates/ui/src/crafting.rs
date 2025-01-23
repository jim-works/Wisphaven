use ahash::HashMap;
use bevy::prelude::*;
use crafting::*;
use engine::{
    actors::LocalPlayer,
    items::{inventory::Inventory, ItemStack},
    world::LevelSystemSet,
    GameState,
};

use crate::{
    inventory::{default_slot_background, InventoryResources, SetIconEvent, SLOT_PX},
    state::UIState,
    styles::get_text_style,
    ButtonColors, ExpandOnHover,
};

pub struct CraftingUIPlugin;

impl Plugin for CraftingUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_crafting_ui,
                update_available_recipes,
                (set_recipe_icon, set_recipe_count),
            )
                .chain()
                .in_set(LevelSystemSet::Main),
        )
        .add_systems(OnEnter(UIState::Inventory), show)
        .add_systems(OnExit(UIState::Inventory), hide);
    }
}

const BASE_HEIGHT_PX: f32 = 36.;
const UI_WIDTH_PX: f32 = 360.;
const RECIPE_MARGIN_ALL_PX: f32 = 4.;
const INPUT_MARGIN_RIGHT_PX: f32 = 2.;
const INPUTS_PER_ROW: usize =
    ((UI_WIDTH_PX - 2. * RECIPE_MARGIN_ALL_PX) / (SLOT_PX + INPUT_MARGIN_RIGHT_PX)) as usize;
const INPUT_MARGIN_BOTTOM_PX: f32 = 2.;

fn spawn_crafting_ui(mut commands: Commands, ui_query: Query<(), With<CraftingUI>>) {
    if !ui_query.is_empty() {
        return;
    }
    commands.spawn((
        Node {
            justify_self: JustifySelf::End,
            position_type: PositionType::Relative,
            width: Val::Px(UI_WIDTH_PX),
            height: Val::Percent(100.),
            border: UiRect::all(Val::Px(2.)),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            ..default()
        },
        Visibility::Hidden,
        BackgroundColor(Color::hsla(0., 0., 0.5, 0.7)),
        BorderColor(Color::hsla(0., 0., 0.3, 1.0)),
        CraftingUI,
        StateScoped(GameState::Game),
    ));
}

#[derive(Component)]
struct CraftingUI;

#[derive(Component)]
struct CraftRecipeButton {
    cached_recipe: Entity,
}

#[derive(Clone)]
struct RecipeRow {
    cached_recipe: Entity,
}

#[derive(Component, Clone)]
struct RecipeItemUI {
    stack: ItemStack,
}

fn update_available_recipes(
    recipe_query: Query<(Entity, &CachedEntityRecipe)>,
    inv_query: Query<&Inventory, (With<LocalPlayer>, Changed<Inventory>)>,
    recipe_row_query: Query<(Entity, &RecipeRow)>,
    ui_root_query: Query<Entity, With<CraftingUI>>,
    mut commands: Commands,
    mut current_recipe_list: Local<HashMap<Entity, Entity>>,
) {
    let Ok(inv) = inv_query.get_single() else {
        return;
    };
    let Ok(ui_root) = ui_root_query.get_single() else {
        warn!("crafting menu not found!");
        return;
    };
    current_recipe_list.clear();
    // map from recipe to UI element
    current_recipe_list.extend(
        recipe_row_query
            .iter()
            .map(|(entity, recipe)| (recipe.cached_recipe, entity)),
    );
    // display a recipe when we have any input
    let available_recipes = recipe_query.iter().filter(|(_, r)| r.has_any_input(inv));
    //the idea is to remove all found recipes, so that current_recipe_list will contain excess recipes for us to remove at the end
    let mut root_ec = commands.entity(ui_root);
    for (recipe_entity, cached_recipe) in available_recipes {
        if current_recipe_list.remove(&recipe_entity).is_none() {
            //UI row for recipe wasn't found, spawn it
            root_ec.with_child((
                Name::new("recipe row"),
                ExpandOnHover {
                    base_height_px: BASE_HEIGHT_PX,
                    extra_height_px: get_input_height_px(cached_recipe.inputs.len()),
                    speed: 10.,
                },
                Node {
                    height: Val::Px(BASE_HEIGHT_PX),
                    margin: UiRect::all(Val::Px(RECIPE_MARGIN_ALL_PX)),
                    align_items: AlignItems::Start,
                    align_content: AlignContent::Start,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::hsla(0., 0., 0.2, 1.0)),
                RecipeRow {
                    cached_recipe: recipe_entity,
                },
                PickingBehavior {
                    // want to be able to scroll the background
                    should_block_lower: false,
                    is_hoverable: true,
                },
            ));
        }
    }

    // now current_recipe_list contains all the excess recipes, so we can despawn them all
    for (_, ui) in current_recipe_list.drain() {
        commands.entity(ui).despawn_recursive();
    }
}

fn get_input_height_px(input_count: usize) -> f32 {
    (SLOT_PX + INPUT_MARGIN_BOTTOM_PX) * input_count.div_ceil(INPUTS_PER_ROW) as f32
}

impl Component for RecipeRow {
    const STORAGE_TYPE: bevy::ecs::component::StorageType =
        bevy::ecs::component::StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_add(|mut world, entity, _| {
            // entity and component should be guaranteed to be alive
            let row = world.get::<RecipeRow>(entity).cloned().unwrap();
            let cached_recipe = world
                .get::<CachedEntityRecipe>(row.cached_recipe)
                .cloned()
                .unwrap();
            let output = cached_recipe.output;
            let output_entity = output.id;
            let output_name = world.get::<Name>(output_entity).unwrap().to_string();
            let resources = world.get_resource::<InventoryResources>().cloned().unwrap();
            let text_style = get_text_style(world.get_resource::<AssetServer>().unwrap());
            let mut commands = world.commands();
            let mut ec = commands.entity(entity);
            ec.with_children(|rows| {
                rows.spawn((
                    Node {
                        height: Val::Px(BASE_HEIGHT_PX),
                        width: Val::Percent(100.),
                        align_content: AlignContent::SpaceBetween,
                        flex_direction: FlexDirection::Row,
                        padding: UiRect::all(Val::Px(2.)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    PickingBehavior {
                        // want to be able to scroll the background
                        should_block_lower: false,
                        is_hoverable: true,
                    },
                ))
                .with_children(|children| {
                    super::inventory::spawn_item_slot(
                        children.spawn_empty(),
                        Node {
                            position_type: PositionType::Relative,
                            margin: UiRect::ZERO,
                            ..default_slot_background()
                        },
                        PickingBehavior {
                            // want to be able to scroll the background
                            should_block_lower: false,
                            is_hoverable: true,
                        },
                        RecipeItemUI { stack: output },
                        &resources,
                    );
                    children.spawn((
                        Node {
                            height: Val::Px(32.),
                            bottom: Val::Px(6.),
                            margin: UiRect::all(Val::Px(2.)),
                            ..default()
                        },
                        Text(output_name),
                        text_style.clone(),
                    ));
                    children
                        .spawn((
                            Button,
                            ButtonColors::default(),
                            BorderColor(ButtonColors::default().default_border),
                            BackgroundColor(ButtonColors::default().default_background),
                            Node {
                                border: UiRect::all(Val::Px(2.0)),
                                align_items: AlignItems::Center,
                                margin: UiRect::left(Val::Auto),
                                height: Val::Px(32.),
                                padding: UiRect::horizontal(Val::Px(5.)),
                                ..default()
                            },
                            CraftRecipeButton {
                                cached_recipe: row.cached_recipe,
                            },
                            PickingBehavior {
                                // want to be able to scroll the background
                                should_block_lower: false,
                                is_hoverable: true,
                            },
                        ))
                        .with_children(|button_text| {
                            button_text.spawn((Text("Craft".to_string()), text_style));
                        })
                        .observe(
                            |click: Trigger<Pointer<Down>>,
                             mut writer: EventWriter<CraftEvent>,
                             button_query: Query<&CraftRecipeButton>,
                             player_query: Query<Entity, With<LocalPlayer>>,
                             recipe_query: Query<&CachedEntityRecipe>| {
                                info!("craft clicked! {:?}", click.entity());
                                if let Ok(button) = button_query.get(click.entity()) {
                                    if let Ok(player) = player_query.get_single() {
                                        // would be cool if I could just send the entity as the event (see comment on CraftEvent)
                                        if let Ok(recipe) = recipe_query.get(button.cached_recipe) {
                                            writer.send(CraftEvent {
                                                crafter: player,
                                                recipe: recipe.clone(),
                                            });
                                        }
                                    }
                                }
                            },
                        );
                });
                rows.spawn((
                    Node {
                        height: Val::Px(get_input_height_px(cached_recipe.inputs.len())),
                        padding: UiRect::horizontal(Val::Px(2.)),
                        align_items: AlignItems::Start,
                        align_content: AlignContent::Start,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        width: Val::Percent(100.),
                        ..default()
                    },
                    PickingBehavior {
                        // want to be able to scroll the background
                        should_block_lower: false,
                        is_hoverable: true,
                    },
                ))
                .with_children(|inputs| {
                    for input in cached_recipe.inputs.iter() {
                        super::inventory::spawn_item_slot(
                            inputs.spawn_empty(),
                            Node {
                                position_type: PositionType::Relative,
                                margin: UiRect::right(Val::Px(2.))
                                    .with_bottom(Val::Px(INPUT_MARGIN_BOTTOM_PX)),
                                ..default_slot_background()
                            },
                            PickingBehavior {
                                // want to be able to scroll the background
                                should_block_lower: false,
                                is_hoverable: true,
                            },
                            RecipeItemUI { stack: *input },
                            &resources,
                        );
                    }
                });
            });
        });
    }
}
fn set_recipe_icon(
    added: Query<(Entity, &RecipeItemUI), (Added<RecipeItemUI>, With<ImageNode>)>,
    mut writer: EventWriter<SetIconEvent>,
) {
    for (icon, item) in added.iter() {
        writer.send(SetIconEvent {
            item: item.stack.id,
            ui_image: icon,
        });
    }
}

fn set_recipe_count(
    mut added: Query<(&RecipeItemUI, &mut Text, &mut Visibility), Added<RecipeItemUI>>,
) {
    for (item, mut text, mut vis) in added.iter_mut() {
        text.0 = item.stack.size.to_string();
        *vis.as_mut() = Visibility::Inherited;
    }
}

fn show(mut query: Query<&mut Visibility, With<CraftingUI>>) {
    for mut vis in query.iter_mut() {
        *vis.as_mut() = Visibility::Inherited;
    }
}

fn hide(mut query: Query<&mut Visibility, With<CraftingUI>>) {
    for mut vis in query.iter_mut() {
        *vis.as_mut() = Visibility::Hidden;
    }
}
