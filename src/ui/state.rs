use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{controllers::Action, actors::LocalPlayer};

#[derive(States, Default, Debug, Hash,PartialEq, Eq, Clone)]
pub enum UIState {
    Hidden,
    #[default]
    Default,
    Inventory
} 

#[derive(States, Default, Debug, Hash,PartialEq, Eq, Clone)]
pub enum DebugUIState {
    Hidden,
    #[default]
    Shown,
} 

pub fn toggle_hidden (
    mut next_state: ResMut<NextState<UIState>>,
    curr_state: Res<State<UIState>>,
    query: Query<&ActionState<Action>, With<LocalPlayer>>,
) {
    if let Ok(action) = query.get_single() {
        if action.just_pressed(Action::ToggleUIHidden) {
            match curr_state.0 {
                UIState::Hidden => next_state.set(UIState::Default),
                _ => next_state.set(UIState::Hidden)
            }
        }
    }
    
}