use bevy::prelude::*;

#[derive(States, Default, Debug, Hash,PartialEq, Eq, Clone)]
pub enum UIState {
    Hidden,
    #[default]
    Default,
    Inventory
} 