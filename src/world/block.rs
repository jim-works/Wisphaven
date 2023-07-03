use std::{ops::AddAssign};

use crate::util::Direction;
use bevy::{prelude::*, utils::HashMap};

use super::chunk::{ChunkCoord, CHUNK_SIZE_I32};

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
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockId {
    #[default]
    Empty,
    Basic(u32),
    Dynamic(u32)
}

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

#[derive(Component)]
pub enum BlockMesh {
    //standard block shape with all sides being the same texture
    Uniform(u32),
    //standard block shape with each side having a unique texture
    MultiTexture([u32; 6]),
    //Slab with height from bottom (1.0) is the same as uniform, (0.0) is empty
    BottomSlab(f32, [u32; 6]),
}

impl BlockMesh {
    pub fn is_mesh_transparent(&self, face: Direction) -> bool {
        match self {
            BlockMesh::Uniform(_) => false,
            BlockMesh::MultiTexture(_) => false,
            BlockMesh::BottomSlab(_, _) => face != Direction::NegY,
        }
    }
}

#[derive(Resource, Default)]
pub struct BlockRegistry {
    pub basic_entities: Vec<Entity>,
    pub dynamic_generators: Vec<Box<dyn BlockGenerator>>,
    //block ids may not be stable across program runs
    pub id_map: HashMap<BlockName, BlockId>
}

impl BlockRegistry {
    pub fn add_basic(&mut self, name: BlockName, entity: Entity) {
        let id = BlockId::Basic(self.basic_entities.len() as u32);
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
        self.add_basic(name, entity);
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
