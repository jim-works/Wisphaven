use std::{ops::AddAssign, sync::Arc};

use crate::util::Direction;
use bevy::{prelude::*, utils::HashMap};
use serde::{Serialize, Deserialize};

use super::chunk::{ChunkCoord, CHUNK_SIZE_I32, ChunkIdx};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockType {
    #[default]
    Empty,
    Filled(Entity)
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, Component)]
pub struct BlockName {
    pub namespace: &'static str,
    pub name: &'static str
}

impl BlockName {
    pub fn new(namespace: &'static str, name: &'static str) -> Self {
        Self {
            namespace,
            name
        }
    }
    //creates a name for the core namespace
    pub fn core(name: &'static str) -> Self {
        Self {
            namespace: "core",
            name
        }
    }
}

//block ids may not be stable across program runs. to get a specific id for a block,
// use block registry
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockId {
    #[default]
    Empty,
    Basic(u32),
    Dynamic(u32)
}

#[derive(Resource, Default)]
pub struct SavedBlockId(pub BlockId);

//used in world generation
//we need this trait because we can't spawn entities in tasks, so we create an instance of BlockGenerator, which we can use later to create the block entity
pub trait BlockGenerator: Send + Sync {
    fn generate(&self, commands: &mut Commands, position: BlockCoord) -> Entity;
}

//marker for components to look for on a BlockEntity
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlockData {
    Storage,
}

#[derive(Component)]
pub struct BlockEntity(Vec<BlockData>);

#[derive(Component, Clone, PartialEq, Default)]
//controls visuals
pub enum BlockMesh {
    //empty mesh
    #[default]
    Empty,
    //standard block shape with all sides being the same texture
    Uniform(u32),
    //standard block shape with each side having a unique texture
    MultiTexture([u32; 6]),
    //Slab with height from bottom (1.0) is the same as uniform, (0.0) is empty
    BottomSlab(f32, [u32; 6]),
}

impl BlockMesh {
    pub fn is_transparent(&self, face: Direction) -> bool {
        match self {
            BlockMesh::Empty => true,
            BlockMesh::Uniform(_) => false,
            BlockMesh::MultiTexture(_) => false,
            BlockMesh::BottomSlab(_, _) => face != Direction::NegY,
        }
    }
}

#[derive(Component, Clone, PartialEq, Default)]
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

#[derive(Resource)]
pub struct BlockResources {
    pub registry: Arc<BlockRegistry>
}

#[derive(Default)]
pub struct BlockRegistry {
    pub basic_entities: Vec<Entity>,
    pub dynamic_generators: Vec<Box<dyn BlockGenerator>>,
    //block ids may not be stable across program runs
    pub id_map: HashMap<BlockName, BlockId>
}

impl BlockRegistry {
    //inserts the corresponding BlockId component on the block
    pub fn add_basic(&mut self, name: BlockName, entity: Entity, commands: &mut Commands) {
        let id = BlockId::Basic(self.basic_entities.len() as u32);
        commands.entity(entity).insert(id);
        self.basic_entities.push(entity);
        self.id_map.insert(name, id);
    }
    pub fn add_dynamic(&mut self, name: BlockName, generator: Box<dyn BlockGenerator>) {
        let id = BlockId::Dynamic(self.dynamic_generators.len() as u32);
        self.dynamic_generators.push(generator);
        self.id_map.insert(name, id);
    }
    pub fn create_basic(&mut self, name: BlockName, mesh: BlockMesh, commands: &mut Commands) {
        let entity = commands.spawn((name, mesh)).id();
        self.add_basic(name, entity, commands);
    }
    pub fn get_basic(&self, name: &BlockName) -> Option<Entity> {
        let id = self.id_map.get(&name)?;
        match id {
            BlockId::Basic(id) => self.basic_entities.get(*id as usize).copied(),
            _ => None
        }
    }
    pub fn get_id(&self, name: &BlockName) -> BlockId {
        match self.id_map.get(name) {
            Some(id) => *id,
            None => {
                error!("Couldn't find block id for name {:?}", name);
                BlockId::Empty
            },
        }
    }
    pub fn get_entity(&self, id: BlockId, position: BlockCoord, commands: &mut Commands) -> Option<Entity> {
        match id {
            BlockId::Empty => None,
            BlockId::Basic(id) => self.basic_entities.get(id as usize).copied(),
            BlockId::Dynamic(id) => self.dynamic_generators.get(id as usize).and_then(|gen| Some(gen.generate(commands, position))),
        }
    }
    pub fn get_block_type(&self, id: BlockId, position: BlockCoord, commands: &mut Commands) -> BlockType {
        match id {
            BlockId::Empty => BlockType::Empty,
            BlockId::Basic(id) => match self.basic_entities.get(id as usize).copied() {
                Some(id) => BlockType::Filled(id),
                None => BlockType::Empty,
            },
            BlockId::Dynamic(id) => match self.dynamic_generators.get(id as usize).and_then(|gen| Some(gen.generate(commands, position))) {
                Some(id) => BlockType::Filled(id),
                None => BlockType::Empty,
            },
        }
    }
    pub fn remove_entity(id_query: &Query<&BlockId>, b: BlockType, commands: &mut Commands) {
        match b {
            BlockType::Filled(entity) => match id_query.get(entity) {
                Ok(BlockId::Empty) | Ok(BlockId::Basic(_)) | Err(_) => {},
                Ok(BlockId::Dynamic(_)) => commands.entity(entity).despawn_recursive(),
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
        BlockCoord::new(
            v.x as i32,
            v.y as i32,
            v.z as i32,
        )
    }
}
