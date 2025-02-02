use bevy::prelude::*;

use world::atmosphere::Calendar;

pub struct StaminaPlugin;

impl Plugin for StaminaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, send_stamina_updated_events)
            .add_systems(FixedUpdate, restore_stamina_during_day)
            .add_event::<StaminaUpdatedEvent>();
    }
}

#[derive(Event, Clone, Copy)]
pub struct StaminaUpdatedEvent {
    pub entity: Entity,
    pub stamina: Stamina,
    pub change: f32,
    pub change_max: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Stamina {
    pub max: f32,
    pub current: f32,
    old_max: f32,
    old_current: f32,
}

impl Default for Stamina {
    fn default() -> Self {
        Self {
            max: 10.0,
            current: 10.0,
            old_max: f32::NEG_INFINITY,
            old_current: f32::NEG_INFINITY,
        }
    }
}

impl Stamina {
    pub fn new(max: f32) -> Self {
        Self {
            max,
            current: max,
            ..Default::default()
        }
    }

    pub fn change(&mut self, amount: f32) {
        self.current += amount;
        self.current = self.current.clamp(0.0, self.max);
    }
}

#[derive(Clone, Copy)]
pub struct StaminaCost {
    cost: f32,
}

impl StaminaCost {
    pub fn new(cost: f32) -> Self {
        Self { cost }
    }

    pub fn can_apply(self, stamina: Stamina) -> bool {
        stamina.current >= self.cost
    }

    pub fn apply(self, stamina: &mut Stamina) -> bool {
        if self.can_apply(*stamina) {
            stamina.current -= self.cost;
            true
        } else {
            false
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct RestoreStaminaDuringDay {
    pub per_tick: f32,
}

pub fn send_stamina_updated_events(
    mut query: Query<(Entity, &mut Stamina), Changed<Stamina>>,
    mut writer: EventWriter<StaminaUpdatedEvent>,
) {
    for (entity, mut stamina) in query.iter_mut() {
        if stamina.old_max != stamina.max || stamina.old_current != stamina.current {
            let change = stamina.current - stamina.old_current;
            let change_max = stamina.max - stamina.old_max;
            stamina.old_max = stamina.max;
            stamina.old_current = stamina.current;
            writer.send(StaminaUpdatedEvent {
                entity,
                stamina: *stamina,
                change_max,
                change,
            });
        }
    }
}

fn restore_stamina_during_day(
    calendar: Res<Calendar>,
    mut query: Query<(&mut Stamina, &RestoreStaminaDuringDay)>,
) {
    if calendar.in_night() {
        return;
    }
    for (mut stamina, restore) in query.iter_mut() {
        stamina.change(restore.per_tick);
    }
}
