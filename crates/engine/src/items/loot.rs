use bevy::prelude::*;

use rand::prelude::*;
use rand_distr::Uniform;

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, _app: &mut App) {
        
    }
}

pub trait LootTableDroppable: Send + Sync + Clone {}
impl<T: Send + Sync + Clone> LootTableDroppable for T {}

#[derive(Component)]
pub struct LootTable<T: LootTableDroppable> {
    //everything is dropped if a drop occurs
    pub drops: Vec<LootTableDrop<T>>,
    pub drop_chance: f32, //0..1
    //if a drop occurs, count is from range. all drops get the same count
    //dropped loot tables are sampled count times
    pub drop_count: Uniform<u32>
}

pub enum LootTableDrop<T: LootTableDroppable> {
    Item(T),
    SubTable(LootTable<T>)
}

impl<T: LootTableDroppable> LootTable<T> {
    pub fn put_loot(&self, dest: &mut Vec<T>) {
        let mut rng = thread_rng();
        if self.drop_chance != 1.0 && rng.sample(Uniform::new(0.,1.)) > self.drop_chance {
            return; //no drop
        }
        let count = self.drop_count.sample(&mut rng) as usize;
        for drop in &self.drops {
            match drop {
                LootTableDrop::Item(i) => dest.extend(itertools::repeat_n(i.clone(), count)),
                LootTableDrop::SubTable(table) => {
                    for _ in 0..count {
                        table.put_loot(dest)
                    }
                },
            }
        }
    }

    pub fn get_loot(&self) -> Vec<T> {
        let mut vec = Vec::new();
        self.put_loot(&mut vec);
        vec
    }
}

impl<T: LootTableDroppable> Default for LootTable<T> {
    fn default() -> Self {
        Self { drops: Default::default(), drop_chance: 1.0, drop_count: Uniform::new_inclusive(1,1) }
    }
}