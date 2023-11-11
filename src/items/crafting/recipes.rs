use bevy::prelude::*;

use crate::{
    items::weapons::MeleeWeaponItem,
    world::{
        events::{BlockHitEvent, ChunkUpdatedEvent},
        BlockId, BlockName, BlockResources, BlockVolume, Level,
    },
};

use super::{CraftingSystemSet, RecipeCandidateEvent, RecipeCraftedEvent};

pub struct RecipesPlugin;

impl Plugin for RecipesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            tnt_recipe_checker.in_set(CraftingSystemSet::RecipeCheckers),
        )
        .add_systems(
            Update,
            tnt_recipe_actor.in_set(CraftingSystemSet::RecipeActor),
        );
    }
}

fn tnt_recipe_checker(
    mut hit_reader: EventReader<BlockHitEvent>,
    item_query: Query<&MeleeWeaponItem>,
    block_query: Query<&BlockName>,
    mut recipe_writer: EventWriter<RecipeCandidateEvent>,
    level: Res<Level>,
) {
    for BlockHitEvent {
        item,
        user: _,
        hit_forward: _,
        block_position,
    } in hit_reader.iter()
    {
        if !item.map(|i| item_query.contains(i)).unwrap_or(false) {
            continue; //not item we care about
        }
        if level
            .get_block_entity(*block_position)
            .map(|e| {
                block_query
                    .get(e)
                    .map(|name| *name == BlockName::core("log"))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
        {
            recipe_writer.send(RecipeCandidateEvent(super::RecipeCraftedEvent {
                volume: BlockVolume::new(*block_position, *block_position),
                id: super::RecipeId(0),
            }))
        }
    }
}

fn tnt_recipe_actor(
    mut reader: EventReader<RecipeCraftedEvent>,
    level: Res<Level>,
    registry: Res<BlockResources>,
    id_query: Query<&BlockId>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    mut commands: Commands,
) {
    let tnt = registry.registry.get_id(&BlockName::core("tnt"));
    for RecipeCraftedEvent { volume, id } in reader.iter() {
        if id.0 == 0 {
            level.set_block(
                volume.min_corner,
                tnt,
                &registry.registry,
                &id_query,
                &mut update_writer,
                &mut commands,
            )
        }
    }
}
