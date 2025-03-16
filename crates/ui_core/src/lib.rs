use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::focus::{HoverMap, PickingInteraction},
    prelude::*,
    window::CursorGrabMode,
};
use debug::TextStyle;
use engine::{camera::MainCamera, controllers::Action};
use leafwing_input_manager::prelude::*;

pub struct UICorePlugin;

impl Plugin for UICorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .insert_resource(UiScale(2.0))
            .add_plugins(bevy_simple_text_input::TextInputPlugin)
            .add_systems(
                Update,
                (
                    change_button_colors,
                    update_main_camera_ui,
                    update_scroll_position,
                    expand_on_hover,
                ),
            );
    }
}

#[derive(Resource)]
pub struct UIFont(pub TextStyle);

#[derive(Component)]
pub struct MainCameraUIRoot;

#[derive(Component, Clone, Copy)]
pub struct ExpandOnHover {
    pub base_height_px: f32,
    pub extra_height_px: f32,
    pub speed: f32,
}

pub fn init(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(UIFont(get_text_style(&asset_server)));
}

//asset_server.load caches, so should be fine
pub fn get_large_text_style(asset_server: &AssetServer) -> TextStyle {
    (
        TextColor::WHITE,
        TextFont {
            font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
            font_size: 64.0,
            ..default()
        },
        PickingBehavior::IGNORE,
    )
}

pub fn get_text_style(asset_server: &AssetServer) -> TextStyle {
    (
        TextColor::WHITE,
        TextFont {
            font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
            font_size: 32.0,
            ..default()
        },
        PickingBehavior::IGNORE,
    )
}

pub fn get_small_text_style(asset_server: &AssetServer) -> TextStyle {
    (
        TextColor::WHITE,
        TextFont {
            font: asset_server.load("fonts/AvenuePixel1.1/TTF/AvenuePixel-Regular.ttf"),
            font_size: 16.0,
            ..default()
        },
        PickingBehavior::IGNORE,
    )
}

pub const TRANSLUCENT_PANEL_BACKGROUND: Color = Color::hsla(272. / 360., 0.15, 0.15, 0.6);

#[derive(Component, Clone)]
pub struct ButtonColors {
    pub default_background: Color,
    pub default_border: Color,
    pub hovered_background: Color,
    pub hovered_border: Color,
    pub pressed_background: Color,
    pub pressed_border: Color,
}

impl Default for ButtonColors {
    fn default() -> Self {
        Self {
            default_background: Color::srgb_u8(70, 130, 50),
            default_border: Color::srgb_u8(37, 86, 46),
            hovered_background: Color::srgb_u8(37, 86, 46),
            hovered_border: Color::srgb_u8(25, 51, 45),
            pressed_background: Color::srgb_u8(23, 32, 56),
            pressed_border: Color::srgb_u8(37, 58, 94),
        }
    }
}

fn change_button_colors(
    mut interaction_query: Query<
        (
            &PickingInteraction,
            &ButtonColors,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (Changed<PickingInteraction>, With<Button>),
    >,
) {
    for (interaction, color, mut background, mut border) in &mut interaction_query {
        match *interaction {
            PickingInteraction::Pressed => {
                background.0 = color.pressed_background;
                border.0 = color.pressed_border;
            }
            PickingInteraction::Hovered => {
                background.0 = color.hovered_background;
                border.0 = color.hovered_border;
            }
            PickingInteraction::None => {
                background.0 = color.default_background;
                border.0 = color.default_border;
            }
        }
    }
}

fn expand_on_hover(
    mut interaction_query: Query<(&PickingInteraction, &ExpandOnHover, &mut Node)>,
    time: Res<Time>,
) {
    for (interaction, expansion, mut node) in &mut interaction_query {
        match *interaction {
            PickingInteraction::Hovered | PickingInteraction::Pressed => {
                if let Val::Px(curr_px) = node.height {
                    //smooth
                    node.height = Val::Px(curr_px.interpolate_stable(
                        &(expansion.base_height_px + expansion.extra_height_px),
                        expansion.speed * time.delta_secs(),
                    ));
                } else {
                    //idk what happens here but just make it px
                    node.height = Val::Px(expansion.base_height_px + expansion.extra_height_px);
                }
            }
            PickingInteraction::None => {
                if let Val::Px(curr_px) = node.height {
                    //smooth
                    node.height = Val::Px(curr_px.interpolate_stable(
                        &expansion.base_height_px,
                        expansion.speed * time.delta_secs(),
                    ));
                } else {
                    //idk what happens here but just make it px
                    node.height = Val::Px(expansion.base_height_px);
                }
            }
        }
    }
}

fn update_main_camera_ui(
    mut commands: Commands,
    camera: Res<MainCamera>,
    ui_query: Query<Entity, With<MainCameraUIRoot>>,
) {
    for ui_element in ui_query.iter() {
        if let Some(mut ec) = commands.get_entity(ui_element) {
            ec.try_insert(TargetCamera(camera.0));
        }
    }
}

/// Updates the scroll position of scrollable nodes in response to mouse input
fn update_scroll_position(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut scrolled_node_query: Query<&mut ScrollPosition>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    const LINE_HEIGHT: f32 = 32.;
    for mouse_wheel_event in mouse_wheel_events.read() {
        let (mut dx, mut dy) = match mouse_wheel_event.unit {
            MouseScrollUnit::Line => (
                mouse_wheel_event.x * LINE_HEIGHT,
                mouse_wheel_event.y * LINE_HEIGHT,
            ),
            MouseScrollUnit::Pixel => (mouse_wheel_event.x, mouse_wheel_event.y),
        };

        if keyboard_input.pressed(KeyCode::ControlLeft)
            || keyboard_input.pressed(KeyCode::ControlRight)
        {
            std::mem::swap(&mut dx, &mut dy);
        }

        for (_pointer, pointer_map) in hover_map.iter() {
            for (entity, _hit) in pointer_map.iter() {
                if let Ok(mut scroll_position) = scrolled_node_query.get_mut(*entity) {
                    scroll_position.offset_x -= dx;
                    scroll_position.offset_y -= dy;
                }
            }
        }
    }
}
