use bevy::prelude::*;

pub(crate) struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<InteractedEvent>();
    }
}

#[derive(Event)]
pub struct InteractedEvent {
    pub user: Entity,
    pub hit_pos: Vec3,
}
