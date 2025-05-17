use bevy::prelude::*;
use dialogue::{DialogueEffect, DialogueEffectEvent};
use engine::items::{ItemName, ItemResources, ItemStack, SpawnDroppedItemEvent};

pub struct DialogueEffectsPlugin;

impl Plugin for DialogueEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, dropped_item.after(dialogue::advance_dialogue));
    }
}

fn dropped_item(
    position_query: Query<&GlobalTransform>,
    mut effect_reader: EventReader<DialogueEffectEvent>,
    mut drop_writer: EventWriter<SpawnDroppedItemEvent>,
    item_resources: Res<ItemResources>,
    mut commands: Commands,
) {
    for effect in effect_reader.read() {
        if let DialogueEffect::GiveItem {
            give_item: drop_item,
            quantity,
        } = &effect.effect
        {
            // try to give item to other person first, fallback to ourselves
            let gtf = match position_query.get(effect.other_entity) {
                Ok(gtf) => gtf,
                Err(_) => match position_query.get(effect.dialogue_entity) {
                    Ok(gtf) => gtf,
                    Err(_) => {
                        error!(
                            "cannot get position for entity to give item from dialgoue effect {:?}",
                            effect.dialogue_entity
                        );
                        continue;
                    }
                },
            };
            match ItemName::try_from(drop_item.as_ref()) {
                Ok(item_name) => {
                    match item_resources
                        .registry
                        .get_entity(item_resources.registry.get_id(&item_name), &mut commands)
                    {
                        Some(item) => {
                            info!("Dropping {} x{} for dialogue event", item_name, quantity);
                            drop_writer.send(SpawnDroppedItemEvent {
                                postion: gtf.translation(),
                                velocity: Vec3::ZERO,
                                stack: ItemStack::new(item, *quantity),
                            });
                        }
                        None => error!("invalid item name `{}`", item_name),
                    }
                }
                Err(e) => error!(
                    "Error parsing item name to drop from dialogue {}: {:?}",
                    drop_item, e
                ),
            }
        }
    }
}
