use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, Serialize, Deserialize)]
pub enum Action {
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Float, //also removed grapples
    Dash,
    Punch,
    Use,
    Look,
    Scroll,
    ToggleInventory,
    ToggleUIHidden,
    ToggleDebugUIHidden,
    ToggleGizmoOverlap,
    ToggleDebugUIDetail,
    ToggleFlight,
    ToggleFullscreen,
}

impl Actionlike for Action {
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            Action::Look => InputControlKind::DualAxis,
            Action::Scroll => InputControlKind::Axis,
            _ => InputControlKind::Button,
        }
    }
}

pub fn get_input_map() -> InputMap<Action> {
    InputMap::default()
        .with(Action::MoveForward, KeyCode::KeyW)
        .with(Action::MoveLeft, KeyCode::KeyA)
        .with(Action::MoveBack, KeyCode::KeyS)
        .with(Action::MoveRight, KeyCode::KeyD)
        .with(Action::MoveUp, KeyCode::Space)
        .with(Action::Dash, KeyCode::ShiftLeft)
        .with(Action::MoveDown, KeyCode::ControlLeft)
        .with(Action::Float, KeyCode::Space)
        .with(Action::ToggleFlight, KeyCode::KeyF)
        .with(Action::Punch, MouseButton::Left)
        .with(Action::Use, MouseButton::Right)
        .with_dual_axis(Action::Look, MouseMove::default())
        .with_axis(Action::Scroll, MouseScrollAxis::Y)
        .with(Action::ToggleInventory, KeyCode::Escape)
        .with(Action::ToggleUIHidden, KeyCode::F1)
        .with(Action::ToggleDebugUIHidden, KeyCode::F3)
        .with(Action::ToggleGizmoOverlap, KeyCode::F4)
        .with(Action::ToggleDebugUIDetail, KeyCode::F5)
        .with(Action::ToggleFullscreen, KeyCode::F11)
}
