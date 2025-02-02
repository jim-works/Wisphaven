use bevy::prelude::*;

use engine::items::HitResult;
use interfaces::scheduling::ItemSystemSet;
use world::atmosphere::{Calendar, SpeedupCalendarEvent};

use engine::items::{UseEndEvent, UseItemEvent};

pub struct TimeItemsPlugin;

impl Plugin for TimeItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            use_skip_to_night_item.in_set(ItemSystemSet::UsageProcessing),
        )
        .register_type::<SkipToNightItem>();
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, FromWorld)]
pub struct SkipToNightItem;

fn use_skip_to_night_item(
    mut reader: EventReader<UseItemEvent>,
    mut hit_writer: EventWriter<UseEndEvent>,
    mut writer: EventWriter<SpeedupCalendarEvent>,
    query: Query<Entity, With<SkipToNightItem>>,
    cal: Res<Calendar>,
) {
    for UseItemEvent {
        user,
        inventory_slot,
        stack,
        tf: _,
    } in reader.read()
    {
        if query.contains(stack.id) {
            info!("Skipping to night...");
            writer.send(SpeedupCalendarEvent(cal.next_night()));
            hit_writer.send(UseEndEvent {
                user: *user,
                inventory_slot: *inventory_slot,
                stack: *stack,
                result: HitResult::Miss,
            });
        }
    }
}
