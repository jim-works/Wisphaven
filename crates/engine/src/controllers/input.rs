use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
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

pub fn get_input_map() -> InputMap<Action> {
    let mut map = InputMap::default();

    map.insert(KeyCode::W, Action::MoveForward);
    map.insert(KeyCode::A, Action::MoveLeft);
    map.insert(KeyCode::S, Action::MoveBack);
    map.insert(KeyCode::D, Action::MoveRight);
    map.insert(KeyCode::Space, Action::MoveUp);
    map.insert(KeyCode::ShiftLeft, Action::Dash);
    map.insert(KeyCode::ControlLeft, Action::MoveDown);
    map.insert(KeyCode::Space, Action::Float);
    map.insert(KeyCode::F, Action::ToggleFlight);

    map.insert(MouseButton::Left, Action::Punch);
    map.insert(MouseButton::Right, Action::Use);

    map.insert(DualAxis::mouse_motion(), Action::Look);

    map.insert(SingleAxis::mouse_wheel_y(), Action::Scroll);
    map.insert(KeyCode::Escape, Action::ToggleInventory);
    map.insert(KeyCode::F1, Action::ToggleUIHidden);
    map.insert(KeyCode::F3, Action::ToggleDebugUIHidden);
    map.insert(KeyCode::F4, Action::ToggleGizmoOverlap);
    map.insert(KeyCode::F5, Action::ToggleDebugUIDetail);
    map.insert(KeyCode::F11, Action::ToggleFullscreen);

    map
}
