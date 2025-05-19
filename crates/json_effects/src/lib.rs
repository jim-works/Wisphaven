use bevy::prelude::*;
use engine::items::{ItemName, ItemResources, ItemStack, SpawnDroppedItemEvent};
use interfaces::scheduling::LevelSystemSet;
use json_interop::{Effect, EffectEvent};

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, dropped_item.in_set(LevelSystemSet::PostTick));
    }
}

fn dropped_item(
    position_query: Query<&GlobalTransform>,
    mut effect_reader: EventReader<EffectEvent>,
    mut drop_writer: EventWriter<SpawnDroppedItemEvent>,
    item_resources: Res<ItemResources>,
    mut commands: Commands,
) {
    for effect in effect_reader.read() {
        if let Effect::GiveItem {
            name: drop_item,
            quantity,
        } = &effect.effect
        {
            // try to give item to other person first, fallback to ourselves
            let Some(gtf) = [effect.secondary_entity, effect.primary_entity]
                .into_iter()
                .filter_map(|opt| opt)
                .filter_map(|e| position_query.get(e).ok())
                .next()
            else {
                error!(
                    "cannot get position for entity to give item from dialgoue effect {:?}",
                    effect
                );
                continue;
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
