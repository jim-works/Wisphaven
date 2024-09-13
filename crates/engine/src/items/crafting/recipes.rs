use bevy::prelude::*;

use util::iterators::*;

use crate::world::{
    events::{BlockHitEvent, ChunkUpdatedEvent},
    BlockCoord, BlockId, BlockName, BlockType, Level,
};

use super::{
    CraftingHammer, CraftingSystemSet, RecipeCandidateEvent, RecipeCraftedEvent, RecipeId,
};

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
        )
        .register_type::<NamedBasicBlockRecipe>()
        .register_type::<Option<BlockName>>()
        .register_type::<Vec<Option<BlockName>>>()
        .register_type::<Vec<Vec<Option<BlockName>>>>()
        .register_type::<Vec<Vec<Vec<Option<BlockName>>>>>()
        .register_type::<(BlockCoord, BlockName)>()
        .register_type::<Vec<(BlockCoord, BlockName)>>();
    }
}

#[derive(Resource)]
pub struct RecipeList {
    pub basic: Vec<BasicBlockRecipe>,
}

impl RecipeList {
    pub fn new(mut basic: Vec<BasicBlockRecipe>) -> Self {
        for (idx, recipe) in basic.iter_mut().enumerate() {
            recipe.id = RecipeId(idx);
        }
        Self { basic }
    }
}

#[derive(Component, Clone, PartialEq, Default, Reflect)]
//loaded from file, converted to BasicBlockRecipe for use in game
#[reflect(Component, FromWorld)]
pub struct NamedBasicBlockRecipe {
    pub recipe: Vec<Vec<Vec<Option<BlockName>>>>,
    pub products: Vec<(BlockCoord, BlockName)>,
}

#[derive(Debug)]
pub struct BasicBlockRecipe {
    //(x,y,z)
    size: (usize, usize, usize),
    recipe: Vec<Option<Entity>>,
    products: Vec<(BlockCoord, BlockType)>, //blocks to place relative to (0,0,0) corner
    pub id: RecipeId,
}

impl BasicBlockRecipe {
    //requires recipe array sizes all > 0 and dimensions to be constant
    //[[[block; x]; y]; z]
    //products: blocks to place relative to (0,0,0) corner
    pub fn new(
        recipe: &Vec<Vec<Vec<Option<Entity>>>>,
        products: Vec<(BlockCoord, BlockType)>,
    ) -> Option<Self> {
        if recipe.is_empty() || recipe[0].is_empty() || recipe[0][0].is_empty() {
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
            id: RecipeId(0),
        })
    }

    pub fn size(&self) -> (usize, usize, usize) {
        self.size
    }

    fn get_recipe_entry(&self, at: BlockCoord) -> Option<Entity> {
        let idx =
            at.x as usize + at.y as usize * self.size.0 + at.z as usize * self.size.0 * self.size.1;
        self.recipe[idx]
    }

    pub fn verify_exact(&self, origin: BlockCoord, level: &Level) -> bool {
        //todo - optimize: it would be faster to get all block entities up front to reduce hashmap queries
        VolumeIterator::new(self.size.0 as u32, self.size.1 as u32, self.size.2 as u32)
            .map(|offset| {
                (
                    BlockCoord::from(offset) + origin,
                    self.get_recipe_entry(BlockCoord::from(offset)),
                )
            })
            .all(|(world_pos, expected)| level.get_block_entity(world_pos) == expected)
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
            VolumeIterator::new(self.size.0 as u32, self.size.1 as u32, self.size.2 as u32) //clear area
                .map(|offset| (BlockCoord::from(offset) + pos, BlockType::Empty))
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

    pub fn get_volume(&self, origin: BlockCoord) -> Volume {
        Volume::new(
            origin.into(),
            (origin + BlockCoord::new(self.size.0 as i32, self.size.1 as i32, self.size.2 as i32))
                .into(),
        )
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
    item_query: Query<&CraftingHammer>,
    mut recipe_writer: EventWriter<RecipeCandidateEvent>,
    level: Res<Level>,
    recipes: Res<RecipeList>,
) {
    for BlockHitEvent {
        item,
        user: _,
        hit_forward: _,
        block_position,
    } in hit_reader.read()
    {
        if !item.map(|i| item_query.contains(i)).unwrap_or(false) {
            continue; //not item we care about
        };

        for recipe in recipes.basic.iter() {
            info!("testing recipe {:?}", recipe);
            if recipe.verify_exact(*block_position, &level) {
                info!("true");
                recipe_writer.send(RecipeCandidateEvent(RecipeCraftedEvent {
                    volume: recipe.get_volume(*block_position),
                    id: recipe.id,
                }));
            }
        }
    }
}

fn basic_recipe_actor(
    mut reader: EventReader<RecipeCraftedEvent>,
    recipes: Res<RecipeList>,
    level: Res<Level>,
    id_query: Query<&BlockId>,
    mut update_writer: EventWriter<ChunkUpdatedEvent>,
    mut commands: Commands,
) {
    for RecipeCraftedEvent { volume, id } in reader.read() {
        if let Some(recipe) = recipes.basic.get(id.0) {
            recipe.spawn_products(
                volume.min_corner.into(),
                &level,
                &id_query,
                &mut update_writer,
                &mut commands,
            );
            info!("crafted recipe with id: {}", id.0);
        }
    }
}
