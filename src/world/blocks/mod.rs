use bevy::prelude::*;

pub struct BlocksPlugin;

pub mod tnt;

impl Plugin for BlocksPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(tnt::TNTPlugin)
        ;
    }
}