use crate::net::NetworkType;

use super::{BlockCoord, Level, BlockId, BlockResources, LevelSystemSet, Id, BlockDamage};
use bevy::prelude::*;

pub struct WorldEventsPlugin;

impl Plugin for WorldEventsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<CreateLevelEvent>()
            .add_event::<OpenLevelEvent>()
            .add_event::<ExplosionEvent>()
            .add_event::<BlockUsedEvent>()
            .add_event::<BlockDamageSetEvent>()
            .add_event::<BlockHitEvent>()
            .add_systems(Update, process_explosions.in_set(LevelSystemSet::Main))
        ;
    }
}

#[derive(Event)]
pub struct CreateLevelEvent {
    pub name: &'static str,
    pub seed: u64,
    pub network_type: NetworkType
}

#[derive(Event)]
pub struct OpenLevelEvent {
    pub name: &'static str,
}

#[derive(Event)]
pub struct BlockUsedEvent {
    pub block_position: BlockCoord,
    pub user: Entity,
    pub use_forward: Vec3,
    pub block_used: Entity,
}

#[derive(Event)]
//triggered when block gets punched
pub struct BlockHitEvent {
    pub item: Option<Entity>,
    pub user: Option<Entity>,
    pub hit_forward: Vec3,
    pub block_position: BlockCoord,
}

#[derive(Event)]
pub struct BlockDamageSetEvent {
    pub block_position: BlockCoord,
    pub damage: BlockDamage,
    pub damager: Option<Entity>
}

#[derive(Event)]
pub struct ExplosionEvent {
    pub radius: f32,
    pub origin: BlockCoord,
}

fn process_explosions(
    mut reader: EventReader<ExplosionEvent>,
    level: Res<Level>,
    mut commands: Commands,
    id_query: Query<&BlockId>,
    resources: Res<BlockResources>,
) {
    for event in reader.iter() {
        let size = event.radius.ceil() as i32;
        let mut changes = Vec::with_capacity((size*size*size) as usize);
        for x in -size..size+1 {
            for y in -size..size+1 {
                for z in -size..size+1 {
                    if x*x+y*y+z*z <= size*size {
                        changes.push((
                            event.origin + BlockCoord::new(x, y, z),
                            BlockId(Id::Empty),
                        ));
                    }
                }
            }
        }
        level.batch_set_block(changes.into_iter(), &resources.registry, &id_query, &mut commands);
    }
}
