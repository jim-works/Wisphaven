mod player_controller;
use leafwing_input_manager::prelude::*;
pub use player_controller::*;

mod input;
pub use input::*;

use bevy::prelude::*;

pub struct ControllersPlugin;

impl Plugin for ControllersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<Action>::default())
        .add_systems((rotate_mouse,jump_player,move_player,follow_local_player))
        ;
    }
}