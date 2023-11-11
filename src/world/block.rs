use std::{ops::AddAssign, path::PathBuf, sync::Arc};

use crate::{
    items::{
        block_item::BlockItem,
        item_attributes::ConsumableItem,
        loot::{LootTable, LootTableDrop},
        CreatorItem, ItemBundle, ItemName, MaxStackSize,
    },
    serialization::BlockTextureMap,
    util::Direction,
};
use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};

use super::{
    chunk::{ChunkCoord, ChunkIdx, CHUNK_SIZE_I32},
    Id,
};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockType {
    #[default]
    Empty,
    Filled(Entity),
}

#[derive(
    Default, Clone, Debug, PartialEq, Eq, Hash, Component, Reflect, Serialize, Deserialize,
)]
#[reflect(Component)]
pub struct BlockName {
    pub namespace: String,
    pub name: String,
}

impl BlockName {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }
    //creates a name for the core namespace
    pub fn core(name: impl Into<String>) -> Self {
        Self {
            namespace: "core".into(),
            name: name.into(),
        }
    }
}

//block ids may not be stable across program runs. to get a specific id for a block,
// use block registry
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub Id);

impl From<Id> for BlockId {
    fn from(value: Id) -> Self {
        Self(value)
    }
}

impl From<BlockId> for Id {
    fn from(value: BlockId) -> Self {
        value.0
    }
}

#[derive(Resource, Default)]
pub struct SavedBlockId(pub BlockId);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct UsableBlock;

//used in world generation
//we need this trait because we can't spawn entities in tasks, so we create an instance of BlockGenerator, which we can use later to create the block entity
pub trait BlockGenerator: Send + Sync {
    fn generate(&self, block: Entity, position: BlockCoord, commands: &mut Commands);
    fn default(&self, block: Entity, commands: &mut Commands);
}

//marker for components to look for on a BlockEntity
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlockData {
    Storage,
}

#[derive(Component)]
pub struct BlockEntity(Vec<BlockData>);

#[derive(Component, Clone, PartialEq, Default, Reflect)]
//controls visuals
//loaded from file, converted to BlockMesh for use in game
#[reflect(Component)]
pub struct NamedBlockMesh {
    pub use_transparent_shader: bool,
    pub shape: NamedBlockMeshShape,
}

impl NamedBlockMesh {
    pub fn to_block_mesh(self, map: &BlockTextureMap) -> BlockMesh {
        BlockMesh {
            use_transparent_shader: self.use_transparent_shader,
            shape: self.shape.to_block_mesh(map),
            single_mesh: None,
        }
    }
}

#[derive(Clone, PartialEq, Default, Reflect)]
pub enum NamedBlockMeshShape {
    //empty mesh
    #[default]
    Empty,
    //standard block shape with all sides being the same texture
    Uniform(PathBuf),
    //standard block shape with each side having a unique texture
    MultiTexture([PathBuf; 6]),
    //Slab with height from bottom (1.0) is the same as uniform, (0.0) is empty
    BottomSlab(f32, [PathBuf; 6]),
    Cross([PathBuf; 2]),
}

impl NamedBlockMeshShape {
    pub fn to_block_mesh(self, map: &BlockTextureMap) -> BlockMeshShape {
        match self {
            NamedBlockMeshShape::Empty => BlockMeshShape::Empty,
            NamedBlockMeshShape::Uniform(name) => {
                BlockMeshShape::Uniform(*map.0.get(&name).unwrap())
            }
            NamedBlockMeshShape::MultiTexture(names) => {
                BlockMeshShape::MultiTexture(names.map(|name| *map.0.get(&name).unwrap()))
            }
            NamedBlockMeshShape::BottomSlab(height, names) => {
                BlockMeshShape::BottomSlab(height, names.map(|name| *map.0.get(&name).unwrap()))
            }
            NamedBlockMeshShape::Cross(names) => {
                BlockMeshShape::Cross(names.map(|name| *map.0.get(&name).unwrap()))
            }
        }
    }
}

#[derive(Component, Default, Clone, PartialEq)]
pub struct BlockMesh {
    pub use_transparent_shader: bool,
    pub shape: BlockMeshShape,
    pub single_mesh: Option<Handle<Mesh>>,
}

#[derive(Clone, PartialEq, Default)]
//controls visuals
pub enum BlockMeshShape {
    //empty mesh
    #[default]
    Empty,
    //standard block shape with all sides being the same texture
    Uniform(u32),
    //standard block shape with each side having a unique texture
    MultiTexture([u32; 6]),
    //Slab with height from bottom (1.0) is the same as uniform, (0.0) is empty
    BottomSlab(f32, [u32; 6]),
    //x-shaped criss-cross (like minecraft flower). each face is a unit square at a 45 degree angle centered in the block
    //technically 4 faces, two for each direction (forward and backwards face) so we don't have to have a special two-sided material
    Cross([u32; 2]),
}

impl BlockMeshShape {
    //if this face of the block does not occlude the entirety of the world behind it
    //sides of slabs, tranparent blocks, etc
    pub fn is_transparent(&self, _face: Direction) -> bool {
        match self {
            BlockMeshShape::Empty => true,
            BlockMeshShape::Uniform(_) => false,
            BlockMeshShape::MultiTexture(_) => false,
            BlockMeshShape::BottomSlab(_, _) => true,
            BlockMeshShape::Cross(_) => true,
        }
    }
}

#[derive(Component, Clone, PartialEq, Default, Reflect)]
#[reflect(Component)]
//controls collider
pub enum BlockPhysics {
    //no collision
    #[default]
    Empty,
    //standard block shape, solid block
    Solid,
    //Slab with height from bottom (1.0) is the same as uniform, (0.0) is empty
    BottomSlab(f32),
}

impl BlockPhysics {
    pub fn has_hole(&self, face: Direction) -> bool {
        match self {
            BlockPhysics::Empty => true,
            BlockPhysics::Solid => false,
            BlockPhysics::BottomSlab(_) => face != Direction::NegY,
        }
    }
}

//0 = healthy, 1 = broken
#[derive(Clone, Copy)]
pub struct BlockDamage {
    pub damage: f32,
    pub seconds_to_next_heal: f32,
}

impl BlockDamage {
    pub const SECONDS_PER_TICK: f32 = 3.0;
    pub const HEAL_PER_TICK: f32 = 0.1;
    pub fn new(damage: f32) -> Self {
        Self {
            damage,
            seconds_to_next_heal: Self::SECONDS_PER_TICK,
        }
    }
    pub fn with_time_reset(self) -> Self {
        Self {
            damage: self.damage,
            seconds_to_next_heal: Self::SECONDS_PER_TICK,
        }
    }
}

impl Default for BlockDamage {
    fn default() -> Self {
        Self {
            damage: Default::default(),
            seconds_to_next_heal: Self::SECONDS_PER_TICK,
        }
    }
}

#[derive(Resource)]
pub struct BlockResources {
    pub registry: Arc<BlockRegistry>,
}

pub type BlockNameIdMap = HashMap<BlockName, BlockId>;

#[derive(Default)]
pub struct BlockRegistry {
    pub basic_entities: Vec<Entity>,
    pub dynamic_generators: Vec<Box<dyn BlockGenerator>>,
    //block ids may not be stable across program runs
    pub id_map: BlockNameIdMap,
}

impl BlockRegistry {
    //inserts the corresponding BlockId component on the block
    pub fn add_basic(&mut self, name: BlockName, entity: Entity, commands: &mut Commands) {
        info!("added block {:?}", name);
        let id = BlockId(Id::Basic(self.basic_entities.len() as u32));
        let item_name = ItemName::core(name.name.clone());
        let item = commands
            .spawn((
                ItemBundle {
                    name: item_name,
                    max_stack_size: MaxStackSize(999),
                },
                BlockItem(entity),
                ConsumableItem,
            ))
            .id();
        commands.entity(entity).insert((
            id,
            LootTable {
                drops: vec![LootTableDrop::Item(item)],
                ..default()
            },
            CreatorItem(item),
        ));
        self.basic_entities.push(entity);
        self.id_map.insert(name, id);
    }
    pub fn add_dynamic(&mut self, name: BlockName, generator: Box<dyn BlockGenerator>) {
        let id = BlockId(Id::Dynamic(self.dynamic_generators.len() as u32));
        self.dynamic_generators.push(generator);
        self.id_map.insert(name, id);
    }
    pub fn get_basic(&self, name: &BlockName) -> Option<Entity> {
        let id = self.id_map.get(name)?;
        match id {
            BlockId(Id::Basic(id)) => self.basic_entities.get(*id as usize).copied(),
            _ => None,
        }
    }
    pub fn get_id(&self, name: &BlockName) -> BlockId {
        match self.id_map.get(name) {
            Some(id) => *id,
            None => {
                error!("Couldn't find block id for name {:?}", name);
                BlockId(Id::Empty)
            }
        }
    }
    pub fn get_entity(&self, block_id: BlockId, commands: &mut Commands) -> Option<Entity> {
        match block_id {
            BlockId(Id::Empty) => None,
            BlockId(Id::Basic(id)) => self.basic_entities.get(id as usize).copied(),
            BlockId(Id::Dynamic(id)) => self.dynamic_generators.get(id as usize).and_then(|gen| {
                let id = Self::setup_block(block_id, commands);
                gen.default(id, commands);
                Some(id)
            }),
        }
    }
    pub fn generate_entity(
        &self,
        block_id: BlockId,
        position: BlockCoord,
        commands: &mut Commands,
    ) -> Option<Entity> {
        match block_id {
            BlockId(Id::Empty) => None,
            BlockId(Id::Basic(id)) => self.basic_entities.get(id as usize).copied(),
            BlockId(Id::Dynamic(id)) => self.dynamic_generators.get(id as usize).and_then(|gen| {
                let id = Self::setup_block(block_id, commands);
                gen.generate(id, position, commands);
                Some(id)
            }),
        }
    }
    fn setup_block(id: BlockId, commands: &mut Commands) -> Entity {
        commands.spawn(id).id()
    }
    pub fn get_block_type(&self, id: BlockId, commands: &mut Commands) -> BlockType {
        match self.get_entity(id, commands) {
            Some(id) => BlockType::Filled(id),
            None => BlockType::Empty,
        }
    }
    pub fn generate_block_type(
        &self,
        id: BlockId,
        pos: BlockCoord,
        commands: &mut Commands,
    ) -> BlockType {
        match self.generate_entity(id, pos, commands) {
            Some(id) => BlockType::Filled(id),
            None => BlockType::Empty,
        }
    }
    pub fn remove_entity(id_query: &Query<&BlockId>, b: BlockType, commands: &mut Commands) {
        match b {
            BlockType::Filled(entity) => match id_query.get(entity) {
                Ok(BlockId(Id::Empty)) | Ok(BlockId(Id::Basic(_))) | Err(_) => {}
                Ok(BlockId(Id::Dynamic(_))) => commands.entity(entity).despawn_recursive(),
            },
            BlockType::Empty => {}
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockCoord {
    pub fn new(x: i32, y: i32, z: i32) -> BlockCoord {
        BlockCoord { x, y, z }
    }
    //returns coordinate at negative corner of block
    pub fn to_vec3(self) -> Vec3 {
        Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }
    //returns coordinate at center of block
    pub fn center(self) -> Vec3 {
        Vec3::new(
            self.x as f32 + 0.5,
            self.y as f32 + 0.5,
            self.z as f32 + 0.5,
        )
    }
    pub fn offset(self, dir: Direction) -> BlockCoord {
        match dir {
            Direction::PosX => BlockCoord::new(self.x + 1, self.y, self.z),
            Direction::PosY => BlockCoord::new(self.x, self.y + 1, self.z),
            Direction::PosZ => BlockCoord::new(self.x, self.y, self.z + 1),
            Direction::NegX => BlockCoord::new(self.x - 1, self.y, self.z),
            Direction::NegY => BlockCoord::new(self.x, self.y - 1, self.z),
            Direction::NegZ => BlockCoord::new(self.x, self.y, self.z - 1),
        }
    }
    //if v has maximum element m, returns the vector with m set to sign(m) and all other elements 0.
    pub fn max_component_norm(self) -> BlockCoord {
        let abs = self.abs();
        if abs.x > abs.y && abs.x > abs.z {
            BlockCoord::new(self.x.signum(), 0, 0)
        } else if abs.y > abs.z {
            BlockCoord::new(0, self.y.signum(), 0)
        } else {
            BlockCoord::new(0, 0, self.z.signum())
        }
    }
    pub fn abs(self) -> BlockCoord {
        BlockCoord::new(self.x.abs(), self.y.abs(), self.z.abs())
    }
}

impl std::ops::Add<BlockCoord> for BlockCoord {
    type Output = Self;
    fn add(self, rhs: BlockCoord) -> Self::Output {
        BlockCoord::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub<BlockCoord> for BlockCoord {
    type Output = Self;
    fn sub(self, rhs: BlockCoord) -> Self::Output {
        BlockCoord::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl AddAssign for BlockCoord {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl std::ops::Mul<i32> for BlockCoord {
    type Output = BlockCoord;

    fn mul(self, rhs: i32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl From<Vec3> for BlockCoord {
    fn from(v: Vec3) -> Self {
        BlockCoord::new(v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32)
    }
}

impl From<ChunkCoord> for BlockCoord {
    fn from(v: ChunkCoord) -> Self {
        BlockCoord::new(
            v.x * CHUNK_SIZE_I32,
            v.y * CHUNK_SIZE_I32,
            v.z * CHUNK_SIZE_I32,
        )
    }
}

impl From<ChunkIdx> for BlockCoord {
    fn from(v: ChunkIdx) -> Self {
        BlockCoord::new(v.x as i32, v.y as i32, v.z as i32)
    }
}

impl From<Direction> for BlockCoord {
    fn from(value: Direction) -> Self {
        match value {
            Direction::PosX => BlockCoord::new(1, 0, 0),
            Direction::PosY => BlockCoord::new(0, 1, 0),
            Direction::PosZ => BlockCoord::new(0, 0, 1),
            Direction::NegX => BlockCoord::new(-1, 0, 0),
            Direction::NegY => BlockCoord::new(0, -1, 0),
            Direction::NegZ => BlockCoord::new(0, 0, -1),
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockVolume {
    pub min_corner: BlockCoord,
    pub max_corner: BlockCoord,
}

impl BlockVolume {
    //returns true if min <= other min and max >= other max.
    //contains itself!
    pub fn contains(&self, other: BlockVolume) -> bool {
        (self.min_corner.x <= other.min_corner.x
            && self.min_corner.y <= other.min_corner.y
            && self.min_corner.z <= other.min_corner.z)
            && (self.max_corner.x >= other.max_corner.x
                && self.max_corner.y >= other.max_corner.y
                && self.max_corner.z >= other.max_corner.z)
    }

    pub fn new(min_corner: BlockCoord, max_corner: BlockCoord) -> Self {
        BlockVolume { min_corner, max_corner }
    }
}
