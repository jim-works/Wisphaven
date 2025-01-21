use bevy::prelude::*;
use engine::items::ItemName;
use serde::{Deserialize, Serialize};

#[derive(
    Default, Clone, Debug, PartialEq, Eq, Hash, Component, Reflect, Serialize, Deserialize,
)]
#[reflect(Component, FromWorld)]
pub struct Recipe {
    pub inputs: Vec<(ItemName, u32)>,
    pub output: (ItemName, u32),
}

pub struct RecipePlugin;

impl Plugin for RecipePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Recipe>();
    }
}
