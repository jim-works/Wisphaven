//disable console window from popping up on windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//have to enable this because it's a nursery feature
#![warn(clippy::disallowed_types)]
//bevy system signatures often violate these rules
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
//TODO: remove this before release. annoying as balls during development
#![allow(dead_code)]
#![feature(assert_matches)]
#![feature(let_chains)]

mod crafting;
pub mod crosshair;
mod game_over;
pub mod inventory;
pub mod main_menu;
pub mod player_stats;
pub mod state;
pub mod styles;
pub mod waves;

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::focus::{HoverMap, PickingInteraction};
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use engine::camera::MainCamera;
use leafwing_input_manager::action_state::ActionState;

use engine::controllers::Action;
use engine::world::LevelSystemSet;
use engine::GameState;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<state::UIState>()
            .add_systems(Startup, styles::init)
            .add_plugins((
                inventory::InventoryPlugin,
                crosshair::CrosshairPlugin,
                player_stats::PlayerStatsUiPlugin,
                waves::WavesPlugin,
                main_menu::MainMenuPlugin,
                game_over::GameOverUIPlugin,
                crafting::CraftingUIPlugin,
            ))
            .add_plugins(bevy_simple_text_input::TextInputPlugin)
            .add_systems(OnEnter(GameState::Game), (state::on_load, capture_mouse))
            .add_systems(OnEnter(state::UIState::Default), capture_mouse)
            .add_systems(OnEnter(state::UIState::Inventory), release_mouse)
            .add_systems(OnExit(GameState::Game), release_mouse)
            .add_systems(Update, state::toggle_hidden.in_set(LevelSystemSet::Main))
            .add_systems(
                Update,
                (
                    toggle_fullscreen,
                    change_button_colors,
                    update_main_camera_ui,
                    update_scroll_position,
                ),
            )
            .insert_resource(UiScale(2.0));
    }
}

#[derive(Resource)]
struct PickingUIBlocker(Entity);

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

pub fn world_mouse_active(state: &state::UIState) -> bool {
    match state {
        state::UIState::Hidden => true,
        state::UIState::Default => true,
        state::UIState::Inventory => false,
    }
}

fn capture_mouse(mut window_query: Query<&mut Window>) {
    let mut window = window_query.get_single_mut().unwrap();
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
}

fn release_mouse(mut window_query: Query<&mut Window>) {
    let mut window = window_query.get_single_mut().unwrap();
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
}

fn toggle_fullscreen(mut window_query: Query<&mut Window>, action: Res<ActionState<Action>>) {
    if action.just_pressed(&Action::ToggleFullscreen) {
        let mut window = window_query.get_single_mut().unwrap();
        window.mode = match window.mode {
            bevy::window::WindowMode::Windowed => {
                bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            }
            _ => bevy::window::WindowMode::Windowed,
        };
    }
}

#[derive(Component)]
pub struct MainCameraUIRoot;

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
