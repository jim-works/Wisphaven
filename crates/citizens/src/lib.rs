use bevy::prelude::*;

mod blacksmith;
mod misguided;
mod wisp;

pub struct CitizensPlugin;

impl Plugin for CitizensPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            wisp::WispPlugin,
            blacksmith::BlacksmithPlugin,
            misguided::MisguidedPlugin,
        ));
    }
}
