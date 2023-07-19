use std::ops::Index;

use bevy::prelude::*;

use super::*;

#[derive(Component)]
pub struct Inventory {
    items: Vec<Option<ItemStack>>,
    owner: Entity,
    selected_slot: usize,
}

impl Inventory {
    pub fn new(owner: Entity, slots: usize) -> Self {
        Self {
            items: vec![None; slots],
            owner,
            selected_slot: 0
        }
    }
    pub fn iter(&self) -> std::slice::Iter<'_, Option<ItemStack>> {
        self.items.iter()
    }
    pub fn selected_slot(&self) -> usize { self.selected_slot }
    pub fn selected_item_entity(&self) -> Option<Entity> { self[self.selected_slot()].as_ref().map(|stack| stack.id)}
    //if slot_num is negative or over the number of slots in the inventory, loop back around 
    pub fn select_slot(&mut self, slot_num: i32, equip_writer: &mut EventWriter<EquipItemEvent>, unequip_writer: &mut EventWriter<UnequipItemEvent>) {
        //loop back around
        let new_slot = slot_num.rem_euclid(self.items.len() as i32) as usize;
        if new_slot != self.selected_slot {
            if let Some(stack) = &self.items[new_slot] {
                equip_writer.send(EquipItemEvent(self.owner, stack.clone()));
            }
            if let Some(stack) = &self.items[self.selected_slot] {
                unequip_writer.send(UnequipItemEvent(self.owner, stack.clone()))
            }
            self.selected_slot = new_slot;
            info!("selected slot {}", self.selected_slot);
        }
        
    }
    pub fn has_item(&self, item: Entity) -> bool {
        self.items.iter().any(|x| if let Some(ref stack) = x {
            stack.id == item
        } else { 
            false
        })
    }
    pub fn count_type(&self, item: Entity) -> usize {
        self.items.iter().filter(|x| if let Some(ref stack) = x {
            stack.id == item
        } else { 
            false
        }).count()
    }
    //returns what's left (if any) of the item stack after picking up
    pub fn pickup_item(&mut self, mut item: ItemStack, data_query: &Query<&MaxStackSize>, pickup_writer: &mut EventWriter<PickupItemEvent>, equip_writer: &mut EventWriter<EquipItemEvent>) -> Option<ItemStack> {
        let initial_size = item.size;
        for i in 0..self.items.len() {
            if item.size == 0 {
                return None;
            }
            let stacks = &mut self.items[i];
            
            match stacks {
                Some(stack) => {
                    //pick up part of the stack
                    if stack.id != item.id {
                        continue;
                    }
                    let picking_up = item.size.min(data_query.get(item.id).unwrap().0-item.size);
                    if picking_up > 0 {
                        item.size -= picking_up;
                        stack.size += picking_up;
                    }
                },
                None => {
                    //pick up the whole stack into an empty slot
                    *stacks = Some(item.clone());
                    pickup_writer.send(PickupItemEvent(self.owner, item.clone()));
                    equip_writer.send(EquipItemEvent(self.owner, item.clone()));
                    item.size = 0;
                    return None
                }
            }
        }
        let picked_up = item.size-initial_size;
        if picked_up > 0 {
            pickup_writer.send(PickupItemEvent(self.owner, ItemStack { id: item.id, size: picked_up }));
            equip_writer.send(EquipItemEvent(self.owner, ItemStack { id: item.id, size: picked_up }));
        }
        Some(item)
    }
    //returns the dropped items
    pub fn drop_slot(&mut self, slot: usize, drop_writer: &mut EventWriter<DropItemEvent>, unequip_writer: &mut EventWriter<UnequipItemEvent>) -> Option<ItemStack> {
        let item = self.items[slot].clone();
        self.items[slot] = None;
        if let Some(ref stack) = item {
            drop_writer.send(DropItemEvent(self.owner, stack.clone()));
            unequip_writer.send(UnequipItemEvent(self.owner, stack.clone()));
        }
        item
    }
    //returns the dropped items
    pub fn drop_items(&mut self, slot: usize, max_drops: u32, drop_writer: &mut EventWriter<DropItemEvent>, unequip_writer: &mut EventWriter<UnequipItemEvent>) -> Option<ItemStack> {
        if let Some(item) = &mut self.items[slot] {
            let to_drop = max_drops.min(item.size);
            if to_drop == item.size {
                drop_writer.send(DropItemEvent(self.owner, item.clone()));
                unequip_writer.send(UnequipItemEvent(self.owner, item.clone()));
                let ret = Some(item.clone());
                self.items[slot] = None;
                ret
            } else {
                item.size -= to_drop;
                drop_writer.send(DropItemEvent(self.owner, item.clone()));
                unequip_writer.send(UnequipItemEvent(self.owner, item.clone()));
                Some(ItemStack {
                    id: item.id,
                    size: to_drop
                })
            }
        } else {
            None
        }
    }
    pub fn use_item(&self, slot: usize, use_pos: GlobalTransform, writer: &mut EventWriter<UseItemEvent>) {
        if let Some(item) = &self.items[slot] {
            writer.send(UseItemEvent(self.owner, item.clone(), use_pos));
        }
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl Index<usize> for Inventory {
    type Output = Option<ItemStack>;

    fn index(&self, index: usize) -> &Self::Output {
        self.items.get(index).unwrap_or(&None)
    }
}