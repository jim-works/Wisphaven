use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use engine::{actors::LocalPlayer, controllers::Action};

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UIState {
    #[default]
    Hidden,
    Default,
    Inventory,
}

pub fn toggle_hidden(
    mut next_state: ResMut<NextState<UIState>>,
    curr_state: Res<State<UIState>>,
    query: Query<&ActionState<Action>, With<LocalPlayer>>,
) {
    if let Ok(action) = query.get_single() {
        if action.just_pressed(Action::ToggleUIHidden) {
            match curr_state.get() {
                UIState::Hidden => next_state.set(UIState::Default),
                _ => next_state.set(UIState::Hidden),
            }
        }
    }
}

pub fn on_load(mut next_state: ResMut<NextState<UIState>>) {
    next_state.set(UIState::Default);
}
