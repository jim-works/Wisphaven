use std::ops::{IndexMut, Index};

use super::{*, chunk::*};

pub enum OctreeNode
{
    Internal(Box<InternalOctreeNode>),
    Leaf(Box<LeafOctreeNode>)
}


#[derive(Clone, Copy, Debug)]
pub struct OctreeNodeData {
    pub position:  OctreeCoord,
    scale: i32,
    level: u8,
}

#[derive(Clone)]
pub struct LeafOctreeNode {
    blocks: [BlockType; BLOCKS_PER_CHUNK],
    pub data: OctreeNodeData,
    pub entity: Entity,
}

#[derive(Clone)]
pub enum LeafType {
    Ungenerated(Entity),
    Full(LeafOctreeNode)
}

pub struct InternalOctreeNode {
    children: [Option<OctreeNode>; 8],
    pub data: OctreeNodeData
}

pub struct Octree {
    root: Box<InternalOctreeNode>,
}

impl Octree {
    pub fn root(&self) -> &Box<InternalOctreeNode> { &self.root }
    pub fn new() -> Octree {
        Octree { root: Box::new(InternalOctreeNode::new(OctreeNodeData::new(0,OctreeCoord { x: 0, y: 0, z: 0 }))) }
    }
    pub fn get(&self, data: OctreeNodeData) -> &Option<OctreeNode> {
        self.root.get_descendant(data)
    }
    pub fn insert(&mut self, node: Box<LeafOctreeNode>) {
        self.update_root(&node.data);
        self.root.add_child(node);
    }
    fn update_root(&mut self, child: &OctreeNodeData) -> bool {
        if self.root.data.contains(child) {
            println!("no new root: {:?}", child);
            return false; //child is already under root, no need to create a new one
        }
        //child is outside root's area, find which direction to expand in
        let dir = Octant::from(child.scaled_pos()-self.root.data.scaled_pos());
        //create new root one level higher recursively update
        let new_pos = (self.root.data.position+dir.to_octree_coord()*2)/2;
        let new_root = Box::new(InternalOctreeNode{children: [None,None,None,None,None,None,None,None], data: OctreeNodeData::new(self.root.data.level+1, new_pos)});
        let old_root = std::mem::replace(&mut self.root, new_root);
        self.root.children[dir.opposite_idx()] = Some(OctreeNode::Internal(old_root));
        println!("new root {:?}", self.root.data);
        return true;
    }
}

impl OctreeNodeData {
    //position of center of node
    pub fn world_pos(&self) -> Vec3 {
        Vec3::new((self.position.x*CHUNK_SIZE_I32*self.scale) as f32,(self.position.y*CHUNK_SIZE_I32*self.scale) as f32,(self.position.z*CHUNK_SIZE_I32*self.scale) as f32)
    }
    pub fn scaled_pos(&self) -> OctreeCoord {
        self.position*self.scale
    }
    pub fn scale(&self) -> i32 { self.scale }
    pub fn level(&self) -> u8 { self.level }
    pub fn child_octant_pos(&self, octant: Octant) -> OctreeCoord {
        self.position*2+octant.to_octree_coord()
    }
    pub fn new(level: u8, position: OctreeCoord) -> Self {
        OctreeNodeData { position, scale: 1<<level , level }
    }
    pub fn contains(&self, other: &OctreeNodeData) -> bool {
        if self.level <= other.level {
            return false;
        }
        //convert coordinates to be the same scale
        let pos_delta = other.scaled_pos()-self.scaled_pos();
        //octree is scale units on each axis, centered on 0, so [-scale/2,scale/2]
        let max_dist = self.scale/2;
        println!("delta {:?}, max_dist {}", pos_delta, max_dist);
        return (-max_dist <= pos_delta.x && pos_delta.x <= max_dist)
            && (-max_dist <= pos_delta.y && pos_delta.y <= max_dist)
            && (-max_dist <= pos_delta.z && pos_delta.z <= max_dist);
    }
}

impl Index<ChunkIdx> for LeafOctreeNode {
    type Output = BlockType;
    fn index(&self, index: ChunkIdx) -> &BlockType {
        &self.blocks[index.to_usize()]
    }
}

impl IndexMut<ChunkIdx> for LeafOctreeNode {
    fn index_mut(&mut self, index: ChunkIdx) -> &mut BlockType {
        &mut self.blocks[index.to_usize()]
    }
}

impl Index<usize> for LeafOctreeNode {
    type Output = BlockType;
    fn index(&self, index: usize) -> &BlockType {
        &self.blocks[index]
    }
}

impl IndexMut<usize> for LeafOctreeNode {
    fn index_mut(&mut self, index: usize) -> &mut BlockType {
        &mut self.blocks[index]
    }
}

impl LeafOctreeNode {
    pub fn new(data: OctreeNodeData, entity: Entity) -> LeafOctreeNode {
        LeafOctreeNode {
            blocks: [BlockType::Empty; BLOCKS_PER_CHUNK],
            entity,
            data
        }
    }
    pub fn block_to_world(&self, position: Vec3) -> Vec3 {
        Vec3::new(((self.data.position.x*CHUNK_SIZE_I32) as f32 +position.x)*(self.data.scale as f32),
        ((self.data.position.y*CHUNK_SIZE_I32) as f32 +position.y)*(self.data.scale as f32),
        ((self.data.position.z*CHUNK_SIZE_I32) as f32 +position.z)*(self.data.scale as f32))
    }
}

impl InternalOctreeNode {
    pub fn get_child(&self, octant: Octant) -> &Option<OctreeNode> { &self.children[octant.to_idx()] }
    //inserts node into the tree, creating internal nodes if needed.
    //if a node in the same position in the tree already exists, it is replaced with the supplied node (or an internal node if it's a leaf node along the path)
    //if the node.level >= self.level, nothing happens
    //otherwise, if the node is too far away, it will still add the child to the closest octant to it, so be careful!
    pub fn add_child(&mut self, node: Box<LeafOctreeNode>) {
        if node.data.level == self.data.level - 1 {
            //direct child of me
            let idx = Octant::from(node.data.position).to_idx();
            self.children[idx] = Some(OctreeNode::Leaf(node))
        } else if node.data.level < self.data.level - 1 {
            //descendant of one of my children
            let child_octant = Octant::from(node.data.position);
            let child_idx = child_octant.to_idx(); 
            match &mut self.children[child_idx] {
                Some(OctreeNode::Internal(i)) => i.add_child(node),
                _ => {
                    let mut internal_node = Self::new(OctreeNodeData::new(self.data.level-1,self.data.child_octant_pos(child_octant)));
                    internal_node.add_child(node);
                    self.children[child_idx] = Some(OctreeNode::Internal(Box::new(internal_node)));
                },
            }
        }
    }
    pub fn get_descendant(&self, id: OctreeNodeData) -> &Option<OctreeNode> { 
        if id.level >= self.data.level { return &None }
        let octant = Octant::from(id.position-self.data.position);
        let child = self.get_child(octant);
        if let Some(OctreeNode::Internal(i)) = child {
            return i.get_descendant(id);
        }
        child
    }
    pub fn new(data: OctreeNodeData) -> InternalOctreeNode {
        InternalOctreeNode { 
            children: [None,None,None,None,None,None,None,None], 
            data 
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OctreeCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32
}

impl std::ops::Add for OctreeCoord {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        OctreeCoord {
            x: self.x+rhs.x,
            y: self.y+rhs.y,
            z: self.z+rhs.z
        }
    }
}

impl std::ops::Sub for OctreeCoord {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        OctreeCoord {
            x: self.x-rhs.x,
            y: self.y-rhs.y,
            z: self.z-rhs.z
        }
    }
}

impl std::ops::Mul<i32> for OctreeCoord {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        OctreeCoord {
            x: self.x*rhs,
            y: self.y*rhs,
            z: self.z*rhs
        }
    }

}
impl std::ops::Div<i32> for OctreeCoord {
    type Output = Self;

    fn div(self, rhs: i32) -> Self::Output {
        OctreeCoord {
            x: self.x/rhs,
            y: self.y/rhs,
            z: self.z/rhs
        }
    }

}
impl OctreeCoord {
    fn from(v: Vec3, level: u32) -> Self {
        OctreeCoord {x:(v.x/(CHUNK_SIZE_F32*((1<<level) as f32))).floor() as i32,
            y:(v.y/(CHUNK_SIZE_F32*((1<<level) as f32))).floor() as i32,
            z:(v.z/(CHUNK_SIZE_F32*((1<<level) as f32))).floor() as i32
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Octant {
    PosXYZ,
    PosXYNegZ,
    PosXZNegY,
    PosXNegYZ,
    NegXPosYZ,
    NegXYPosZ,
    NegXZPosY,
    NegXYZ
}

impl Octant {
    pub fn to_idx(&self) -> usize {
        match self {
            Octant::PosXYZ => 0,
            Octant::PosXYNegZ => 1,
            Octant::PosXZNegY => 2,
            Octant::PosXNegYZ => 3,
            Octant::NegXPosYZ => 4,
            Octant::NegXZPosY => 5,
            Octant::NegXYPosZ => 6,
            Octant::NegXYZ => 7,
        }
    }

    pub fn to_octree_coord(&self) -> OctreeCoord {
        match self {
            Octant::PosXYZ => OctreeCoord { x: 1, y: 1, z: 1 }, 
            Octant::PosXYNegZ => OctreeCoord { x: 1, y: 1, z: -1 },
            Octant::PosXZNegY => OctreeCoord { x: 1, y: -1, z: 1 },
            Octant::PosXNegYZ => OctreeCoord { x: 1, y: -1, z: -1 },
            Octant::NegXPosYZ => OctreeCoord { x: -1, y: 1, z: 1 },
            Octant::NegXYPosZ => OctreeCoord { x: -1, y: -1, z: 1 },
            Octant::NegXZPosY => OctreeCoord { x: -1, y: 1, z: -1 },
            Octant::NegXYZ => OctreeCoord { x: -1, y: -1, z: -1 },
        }
    }
    //returns index of negative of the octant
    //Ex: NegXPosYZ returns PosXNegYZ
    pub fn opposite_idx(&self) -> usize {
        7-self.to_idx()
    }
    pub fn iter() -> OctantIterator {OctantIterator { curr: None }}
}
impl From<ChunkCoord> for Octant {
    fn from(value: ChunkCoord) -> Self {
        if value.x < 0 {
            if value.y < 0 {
                if value.z < 0 {
                    Octant::NegXYZ
                } else {
                    Octant::NegXYPosZ
                }
            } else {
                if value.z < 0 {
                    Octant::NegXZPosY
                } else {
                    Octant::NegXPosYZ
                }
            }
        } else {
            if value.y < 0 {
                if value.z < 0 {
                    Octant::PosXNegYZ
                } else {
                    Octant::PosXZNegY
                }
            } else {
                if value.z < 0 {
                    Octant::PosXYNegZ
                } else {
                    Octant::PosXYZ
                }
            }
        }
    }
}
impl From<OctreeCoord> for Octant {
    fn from(value: OctreeCoord) -> Self {
        if value.x < 0 {
            if value.y < 0 {
                if value.z < 0 {
                    Octant::NegXYZ
                } else {
                    Octant::NegXYPosZ
                }
            } else {
                if value.z < 0 {
                    Octant::NegXZPosY
                } else {
                    Octant::NegXPosYZ
                }
            }
        } else {
            if value.y < 0 {
                if value.z < 0 {
                    Octant::PosXNegYZ
                } else {
                    Octant::PosXZNegY
                }
            } else {
                if value.z < 0 {
                    Octant::PosXYNegZ
                } else {
                    Octant::PosXYZ
                }
            }
        }
    }
}
pub struct OctantIterator {
    curr: Option<Octant>
}
impl Iterator for OctantIterator {
    type Item = Octant;
    fn next(&mut self) -> Option<Self::Item> {
        self.curr = match self.curr {
            None => Some(Octant::PosXYZ),
            Some(Octant::PosXYZ) => Some(Octant::PosXYNegZ),
            Some(Octant::PosXYNegZ) => Some(Octant::PosXZNegY),
            Some(Octant::PosXZNegY) => Some(Octant::PosXNegYZ),
            Some(Octant::PosXNegYZ) => Some(Octant::NegXPosYZ),
            Some(Octant::NegXPosYZ) => Some(Octant::NegXYPosZ),
            Some(Octant::NegXYPosZ) => Some(Octant::NegXZPosY),
            Some(Octant::NegXZPosY) => Some(Octant::NegXYZ),
            Some(Octant::NegXYZ) => None,
        };
        self.curr
    }
}