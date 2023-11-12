use bevy::prelude::*;

use crate::{
    items::weapons::MeleeWeaponItem,
    util::iterators::BlockVolumeIterator,
    world::{
        events::{BlockHitEvent, ChunkUpdatedEvent},
        BlockCoord, BlockId, BlockName, BlockResources, BlockType, BlockVolume, Level,
    },
};

use super::{CraftingSystemSet, RecipeCandidateEvent, RecipeCraftedEvent};

pub struct RecipesPlugin;

impl Plugin for RecipesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            basic_recipe_checker.in_set(CraftingSystemSet::RecipeCheckers),
        )
        .add_systems(
            Update,
            basic_recipe_actor.in_set(CraftingSystemSet::RecipeActor),
        );
    }
}

pub struct RecipeList {
    pub basic: Vec<BasicBlockRecipe>
}

pub struct BasicBlockRecipe {
    //(x,y,z)
    size: (usize, usize, usize),
    recipe: Vec<Option<Entity>>,
    products: Vec<(BlockCoord, BlockType)>, //blocks to place relative to (0,0,0) corner
}

impl BasicBlockRecipe {
    //requires recipe array sizes all > 0 and dimensions to be constant
    //[[[block; x]; y]; z]
    //products: blocks to place relative to (0,0,0) corner
    pub fn new(
        recipe: &Vec<Vec<Vec<Option<Entity>>>>,
        products: Vec<(BlockCoord, BlockType)>,
    ) -> Option<Self> {
        if recipe.len() == 0 || recipe[0].len() == 0 || recipe[0][0].len() == 0 {
            error!("Recipe length was 0 on an axis");
            return None;
        }
        let size = (recipe[0][0].len(), recipe[0].len(), recipe.len());
        let mut flat_recipe: Vec<Option<Entity>> =
            Vec::with_capacity(recipe[0][0].len() * recipe[0].len() * recipe.len());
        for z in recipe.iter() {
            if z.len() != size.1 {
                error!("Recipe entry on y axis has non-uniform size");
                return None;
            }
            for y in z.iter() {
                if y.len() != size.0 {
                    error!("Recipe entry on x axis has non-uniform size");
                    return None;
                }
                flat_recipe.extend(y.iter());
            }
        }
        Some(Self {
            size,
            recipe: flat_recipe,
            products,
        })
    }

    pub fn size(&self) -> (usize, usize, usize) {
        self.size
    }

    //blocks in [[[option<entity>; x]; y]; z]
    pub fn verify_exact(&self, blocks: &Vec<Vec<Vec<Option<Entity>>>>) -> bool {
        if blocks.len() != self.size.2 {
            return false; //z size doesn't match
        }
        for (iz, z) in blocks.iter().enumerate() {
            if z.len() != self.size.1 {
                return false; //y size doesn't match
            }
            for (iy, y) in z.iter().enumerate() {
                if y.len() != self.size.0 {
                    return false; //x size doesn't match
                }
                for (ix, block) in y.iter().enumerate() {
                    if self[(ix, iy, iz)] != *block {
                        return false; //block doesn't match
                    }
                }
            }
        }
        true
    }

    pub fn spawn_products(
        &self,
        pos: BlockCoord,
        level: &Level,
        id_query: &Query<&BlockId>,
        update_writer: &mut EventWriter<ChunkUpdatedEvent>,
        commands: &mut Commands,
    ) {
        level.batch_set_block_entities(
            BlockVolumeIterator::new(self.size.0 as u32, self.size.1 as u32, self.size.2 as u32) //clear area
                .map(|offset| (offset + pos, BlockType::Empty))
                .chain(
                    //spawn products
                    self.products
                        .iter()
                        .map(|(offset, product)| (*offset + pos, *product)),
                ),
            id_query,
            update_writer,
            commands,
        );
    }
}

impl std::ops::Index<(usize, usize, usize)> for BasicBlockRecipe {
    type Output = Option<Entity>;

    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        let size = self.size();
        &self.recipe[index.0 + index.1 * size.0 + index.2 * size.0 * size.1]
    }
}

fn basic_recipe_checker(
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

fn basic_recipe_actor(
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
