use bevy::prelude::*;

pub mod dash;

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(dash::DashPlugin)
            .add_systems(Update, send_stamina_updated_events)
            .add_event::<StaminaUpdatedEvent>();
    }
}

#[derive(Event, Clone, Copy)]
pub struct StaminaUpdatedEvent(pub Entity, pub Stamina);

#[derive(Component, Clone, Copy, Debug)]
pub struct Stamina {
    pub max: f32,
    pub current: f32,
    old_max: f32,
    old_current: f32
}

impl Default for Stamina {
    fn default() -> Self {
        Self { max: 10.0, current: 10.0, old_max: f32::NEG_INFINITY, old_current: f32::NEG_INFINITY }
    }
}

impl Stamina {
    fn new(max: f32) -> Self {
        Self { max, current: max, ..Default::default() }
    }
}

#[derive(Clone, Copy)]
pub struct StaminaCost {
    cost: f32
}

impl StaminaCost {
    pub fn new(cost: f32) -> Self {
        Self {
            cost,
        }
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

pub fn send_stamina_updated_events(
    mut query: Query<(Entity, &mut Stamina), Changed<Stamina>>,
    mut writer: EventWriter<StaminaUpdatedEvent>
) {
    for (entity, mut stamina) in query.iter_mut() {
        info!("changed {:?}", stamina);
        if stamina.old_max != stamina.max || stamina.old_current != stamina.current {
            info!("changed2");
            stamina.old_max = stamina.max;
            stamina.old_current = stamina.current;
            writer.send(StaminaUpdatedEvent(entity, *stamina));
        }
    }
}