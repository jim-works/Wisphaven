use bevy::prelude::*;

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