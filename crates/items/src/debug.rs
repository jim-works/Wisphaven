use bevy::prelude::*;
use engine::{
    actors::{DamageTakenEvent, DeathTrigger},
    items::UseItemEvent,
};

pub(crate) struct DebugItems;

impl Plugin for DebugItems {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, use_suicide_pill)
            .register_type::<SuicidePill>();
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, FromWorld)]
struct SuicidePill;

fn use_suicide_pill(
    mut reader: EventReader<UseItemEvent>,
    mut attack_writer: EventWriter<DeathTrigger>,
    item_query: Query<&SuicidePill>,
) {
    for UseItemEvent {
        user,
        inventory_slot: _,
        stack,
        tf: _,
    } in reader.read()
    {
        if item_query.contains(stack.id) {
            attack_writer.send(DeathTrigger {
                final_blow: DamageTakenEvent {
                    target: *user,
                    attacker: Some(Entity::PLACEHOLDER),
                    damage: engine::actors::Damage::new(0.0),
                    knockback_impulse: Vec3::ZERO,
                    hit_location: Vec3::ZERO,
                },
                damage_taken: 0.0,
            });
        }
    }
}
