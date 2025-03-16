use bevy::prelude::*;
use lightyear::prelude::ClientId;
use serde::{Deserialize, Serialize};

//ids may not be stable across program runs. to get a specific id for an entity or name,
// use the corresponding registry. DO NOT HARDCODE (unless the backing id dict is hardcoded)
#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Id {
    #[default]
    Empty,
    Basic(u32),
    Dynamic(u32),
}

impl Id {
    pub fn with_id(self, new_id: u32) -> Self {
        match self {
            Id::Empty => Id::Empty,
            Id::Basic(_) => Id::Basic(new_id),
            Id::Dynamic(_) => Id::Dynamic(new_id),
        }
    }
}

#[derive(Component, Default)]
pub struct DebugDrawTransform;

#[derive(Debug)]
pub enum HandState {
    Following,
    Windup {
        start_pos: Vec3,
        windup_time: f32,
        time_remaining: f32,
    },
    Hitting {
        start_pos: Vec3,
        target: Vec3,
        hit_time: f32,
        return_time: f32,
        hit_time_remaining: f32,
    },
    Returning {
        start_pos: Vec3,
        return_time: f32,
        return_time_remaining: f32,
    },
}

#[derive(Component)]
pub struct Hand {
    pub owner: Entity,
    pub offset: Vec3,
    pub scale: f32,
    pub rotation: Quat,
    pub windup_offset: Vec3,
    pub state: HandState,
}

#[derive(Component)]
pub struct SwingHand {
    pub hand: Entity,
    //offset to play hit animation at if the swing misses
    pub miss_offset: Vec3,
}

#[derive(Component)]
pub struct UseHand {
    pub hand: Entity,
    //offset to play hit animation at if the item doesn't have a use coord (e.g. throwing a coin or not placing a block)
    pub miss_offset: Vec3,
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
pub struct RemoteClient(pub ClientId);

#[derive(Component)]
pub struct VisualizeHeldItem {
    pub inventory: Entity,
}
