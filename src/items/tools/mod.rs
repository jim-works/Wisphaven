use bevy::prelude::*;
use serde::{Serialize, Deserialize};

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Tool>()
            .register_type::<ToolPower>()
        ;
    }
}

#[derive(Clone, Hash, Eq, Debug, PartialEq, Component, FromReflect, Reflect, Default, Serialize, Deserialize)]
pub enum ToolPower {
    #[default]
    None,
    Axe(u32),
    Pickaxe(u32),
    Shovel(u32),
}

#[derive(Clone, Hash, Eq, Debug, PartialEq, Component, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Tool {
    pub powers: Vec<ToolPower>,
}