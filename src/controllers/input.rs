use bevy::{prelude::*, reflect::TypePath};
use leafwing_input_manager::prelude::*;

#[derive(TypePath, Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Action {
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Punch,
    Use,
    Look,
    Scroll,
    ToggleInventory,
    ToggleUIHidden,
    ToggleDebugUIHidden,
}

pub fn get_input_map() -> InputMap<Action> {
    let mut map = InputMap::default();

    map.insert(KeyCode::W, Action::MoveForward);
    map.insert(KeyCode::A, Action::MoveLeft);
    map.insert(KeyCode::S, Action::MoveBack);
    map.insert(KeyCode::D, Action::MoveRight);
    map.insert(KeyCode::Space, Action::MoveUp);
    map.insert(KeyCode::ShiftLeft, Action::MoveDown);
    // map.insert(KeyCode::Space, Action::Jump);

    map.insert(MouseButton::Left, Action::Punch);
    map.insert(MouseButton::Right, Action::Use);

    map.insert(DualAxis::mouse_motion(), Action::Look);

    map.insert(SingleAxis::mouse_wheel_y(), Action::Scroll);
    map.insert(KeyCode::Escape, Action::ToggleInventory);
    map.insert(KeyCode::F1, Action::ToggleUIHidden);
    map.insert(KeyCode::F3, Action::ToggleDebugUIHidden);

    map
}