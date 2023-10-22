use bevy::prelude::*;

use crate::world::{LevelSystemSet, atmosphere::{SpeedupCalendarEvent, Calendar}};

use super::UseItemEvent;

pub struct TimeItemsPlugin;

impl Plugin for TimeItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, use_skip_to_night_item.in_set(LevelSystemSet::Main))
            .register_type::<SkipToNightItem>();
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct SkipToNightItem;

fn use_skip_to_night_item(
    reader: EventReader<UseItemEvent>,
    mut writer: EventWriter<SpeedupCalendarEvent>,
    cal: Res<Calendar>
) {
    if !reader.is_empty() {
        info!("Skipping to night...");
        writer.send(SpeedupCalendarEvent(cal.next_night()));
    }
}