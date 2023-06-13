use bevy::prelude::*;

use super::*;

#[derive(Component)]
pub struct Inventory {
    pub items: Vec<Option<ItemStack>>,
    owner: Entity
}

impl Inventory {
    pub fn new(owner: Entity, slots: usize) -> Self {
        Self {
            items: vec![None; slots],
            owner
        }
    }
    pub fn has_item(&self, item: ItemType) -> bool {
        self.items.iter().any(|x| if let Some(ref stack) = x {
            stack.id == item
        } else { 
            false
        })
    }
    pub fn count_type(&self, item: ItemType) -> usize {
        self.items.iter().filter(|x| if let Some(ref stack) = x {
            stack.id == item
        } else { 
            false
        }).count()
    }
    //returns what's left (if any) of the item stack after picking up
    pub fn pickup_item(&mut self, mut item: ItemStack, registry: &ItemRegistry, writer: &mut EventWriter<PickupItemEvent>) -> Option<ItemStack> {
        let initial_size = item.size;
        for i in 0..self.items.len() {
            if item.size == 0 {
                return None;
            }
            let stacks = &mut self.items[i];
            match stacks {
                Some(stack) => {
                    let picking_up = item.size.min(registry.get_data(&item.id).unwrap().max_stack_size-item.size);
                    if picking_up > 0 {
                        item.size -= picking_up;
                        stack.size += picking_up;
                    }
                },
                None => {
                    *stacks = Some(item.clone());
                    writer.send(PickupItemEvent(self.owner, item.clone()));
                    item.size = 0;
                    return None
                }
            }
        }
        let picked_up = item.size-initial_size;
        if picked_up > 0 {
            writer.send(PickupItemEvent(self.owner, ItemStack { id: item.id, size: picked_up }));
        }
        return Some(item);
    }
    //returns the dropped items
    pub fn drop_slot(&mut self, slot: usize, writer: &mut EventWriter<DropItemEvent>) -> Option<ItemStack> {
        let item = self.items[slot].clone();
        self.items[slot] = None;
        if let Some(ref stack) = item {
            writer.send(DropItemEvent(self.owner, stack.clone()))
        }
        return item;
    }
    //returns the dropped items
    pub fn drop_items(&mut self, slot: usize, max_drops: u32, writer: &mut EventWriter<DropItemEvent>) -> Option<ItemStack> {
        if let Some(item) = &mut self.items[slot] {
            let to_drop = max_drops.min(item.size);
            if to_drop == item.size {
                writer.send(DropItemEvent(self.owner, item.clone()));
                let ret = Some(item.clone());
                drop(item);
                self.items[slot] = None;
                return ret;
            } else {
                item.size -= to_drop;
                writer.send(DropItemEvent(self.owner, item.clone()));
                return Some(ItemStack {
                    id: item.id,
                    size: to_drop
                });
            }
        } else {
            None
        }
    }
    pub fn use_item(&self, slot: usize, use_pos: GlobalTransform, writer: &mut EventWriter<UseItemEvent>) {
        if let Some(item) = &self.items[slot] {
            writer.send(UseItemEvent(self.owner, item.id, use_pos));
        }
    }
}