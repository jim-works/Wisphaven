use bevy::{prelude::*, time::Stopwatch};

use super::*;

#[derive(Default, Clone)]
pub enum ItemAction {
    #[default]
    None,
    //elapsed time, target position
    UsingWindup(Stopwatch, GlobalTransform),
    UsingBackswing(Stopwatch),
    //elapsed time, target position
    SwingingWindup(Stopwatch, GlobalTransform),
    SwingingBackswing(Stopwatch)
}

impl ItemAction {
    //will cancel any windup
    //doesn't affect backswing - we don't want the player to be able to switch slots to cancel it
    pub fn cancel_action(&mut self) {
        match self {
            ItemAction::None => (),
            ItemAction::UsingWindup(_,_) => *self = ItemAction::None,
            ItemAction::UsingBackswing(_) => (),
            ItemAction::SwingingWindup(_,_) => *self = ItemAction::None,
            ItemAction::SwingingBackswing(_) => (),
        }
    }
    pub fn try_swing(&mut self, tf: GlobalTransform) {
        match self {
            ItemAction::None => *self = ItemAction::SwingingWindup(default(), tf),
            _ => ()
        }
    }
    pub fn try_use(&mut self, tf: GlobalTransform) {
        match self {
            ItemAction::None => *self = ItemAction::UsingWindup(default(), tf),
            _ => ()
        }
    }
}

#[derive(Component)]
pub struct Inventory {
    items: Vec<Option<(ItemStack, ItemAction)>>,
    owner: Entity,
    selected_slot: usize,
}

impl Inventory {
    pub fn new(owner: Entity, slots: usize) -> Self {
        Self {
            items: vec![None; slots],
            owner,
            selected_slot: 0,
        }
    }
    pub fn iter(&self) -> std::slice::Iter<'_, Option<(ItemStack, ItemAction)>> {
        self.items.iter()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Option<(ItemStack, ItemAction)>> {
        self.items.iter_mut()
    }
    pub fn selected_slot(&self) -> usize { self.selected_slot }
    pub fn selected_item_entity(&self) -> Option<Entity> { self.selected_item().map(|stack| stack.id)}
    pub fn selected_item(&self) -> Option<ItemStack> { self.get(self.selected_slot()) }
    //if slot_num is negative or over the number of slots in the inventory, loop back around 
    pub fn select_slot(&mut self, slot_num: i32, equip_writer: &mut EventWriter<EquipItemEvent>, unequip_writer: &mut EventWriter<UnequipItemEvent>) {
        //loop back around
        let new_slot = slot_num.rem_euclid(self.items.len() as i32) as usize;
        if new_slot != self.selected_slot {
            if let Some((stack, action)) = &mut self.items[new_slot] {
                action.cancel_action();
                equip_writer.send(EquipItemEvent(self.owner, *stack));
            }
            if let Some((stack, action)) = &mut self.items[self.selected_slot] {
                action.cancel_action();
                unequip_writer.send(UnequipItemEvent(self.owner, *stack))
            }
            self.selected_slot = new_slot;
            info!("selected slot {}", self.selected_slot);
        }
        
    }
    pub fn has_item(&self, item: Entity) -> bool {
        self.items.iter().any(|x| if let Some(ref stack) = x {
            stack.0.id == item
        } else { 
            false
        })
    }
    pub fn count_type(&self, item: Entity) -> usize {
        self.items.iter().filter(|x| if let Some(ref stack) = x {
            stack.0.id == item
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
                    if stack.0.id != item.id {
                        continue;
                    }
                    let picking_up = item.size.min(data_query.get(item.id).unwrap().0-item.size);
                    if picking_up > 0 {
                        item.size -= picking_up;
                        stack.0.size += picking_up;
                    }
                },
                None => {
                    //pick up the whole stack into an empty slot
                    *stacks = Some((item, default()));
                    pickup_writer.send(PickupItemEvent(self.owner, item));
                    equip_writer.send(EquipItemEvent(self.owner, item));
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
            drop_writer.send(DropItemEvent(self.owner, stack.0));
            unequip_writer.send(UnequipItemEvent(self.owner, stack.0));
        }
        item.map(|(stack,_)| stack)
    }
    //returns the dropped items
    pub fn drop_items(&mut self, slot: usize, max_drops: u32, drop_writer: &mut EventWriter<DropItemEvent>, unequip_writer: &mut EventWriter<UnequipItemEvent>) -> Option<ItemStack> {
        if let Some(item) = &mut self.items[slot] {
            let to_drop = max_drops.min(item.0.size);
            if to_drop == item.0.size {
                drop_writer.send(DropItemEvent(self.owner, item.0));
                unequip_writer.send(UnequipItemEvent(self.owner, item.0));
                let ret = Some(item.0);
                self.items[slot] = None;
                ret
            } else {
                item.0.size -= to_drop;
                drop_writer.send(DropItemEvent(self.owner, item.0));
                unequip_writer.send(UnequipItemEvent(self.owner, item.0));
                Some(ItemStack {
                    id: item.0.id,
                    size: to_drop
                })
            }
        } else {
            None
        }
    }
    pub fn use_item(&mut self, slot: usize, use_pos: GlobalTransform) {
        if let Some((_,action)) = &mut self.items[slot] {
            action.try_use(use_pos);
        }
    }
    pub fn swing_item(&mut self, slot: usize, use_pos: GlobalTransform) {
        if let Some((_,action)) = &mut self.items[slot] {
            action.try_swing(use_pos);
        }
    }
    pub fn get(&self, slot: usize) -> Option<ItemStack> {
        self.items.get(slot).cloned().flatten().map(|(stack, _)| stack)
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

pub fn tick_item_timers (
    mut query: Query<&mut Inventory>,
    swing_speed_query: Query<&ItemSwingSpeed>,
    use_speed_query: Query<&ItemUseSpeed>,
    time: Res<Time>,
    mut use_writer: EventWriter<UseItemEvent>,
    mut swing_writer: EventWriter<SwingItemEvent>,
) {
    for mut inventory in query.iter_mut() {
        let owner = inventory.owner;
        for opt in inventory.iter_mut() {
            if let Some((stack, action)) = opt {
                match action {
                    ItemAction::None => (),
                    ItemAction::UsingWindup(elapsed, use_pos) => {
                        elapsed.tick(time.delta());
                        match use_speed_query.get(stack.id) {
                            Ok(use_speed) =>  {
                                if elapsed.elapsed() >= use_speed.windup {
                                    use_writer.send(UseItemEvent(owner, *stack, *use_pos));
                                    *action = ItemAction::UsingBackswing(default());
                                }
                            },
                            _ => {
                                use_writer.send(UseItemEvent(owner, *stack, *use_pos));
                                *action = ItemAction::None;
                            }
                        }
                    },
                    ItemAction::UsingBackswing(elapsed) => {
                        elapsed.tick(time.delta());
                        match use_speed_query.get(stack.id) {
                            Ok(use_speed) => {
                                if elapsed.elapsed() >= use_speed.backswing {
                                    *action = ItemAction::None;
                                }
                            },
                            _ => *action = ItemAction::None
                        }
                    },
                    ItemAction::SwingingWindup(elapsed, use_pos) => {
                        elapsed.tick(time.delta());
                        match swing_speed_query.get(stack.id) {
                            Ok(swing_speed) =>  {
                                if elapsed.elapsed() >= swing_speed.windup {
                                    swing_writer.send(SwingItemEvent(owner, *stack, *use_pos));
                                    *action = ItemAction::SwingingBackswing(default());
                                }
                            },
                            _ => {
                                swing_writer.send(SwingItemEvent(owner, *stack, *use_pos));
                                *action = ItemAction::None;
                            }
                        }
                    },
                    ItemAction::SwingingBackswing(elapsed) => {
                        elapsed.tick(time.delta());
                        match swing_speed_query.get(stack.id) {
                            Ok(swing_speed) =>  {
                                if elapsed.elapsed() >= swing_speed.windup {
                                *action = ItemAction::None;
                                }
                            },
                            _ => *action = ItemAction::None
                        }
                    },
                }
            }
        }
    }
}