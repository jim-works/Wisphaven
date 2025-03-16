use bevy::{prelude::*, time::Stopwatch};

use crate::util::ExtraOptions;

use super::{
    item_attributes::{ItemSwingSpeed, ItemUseSpeed},
    *,
};

#[derive(Clone, Copy)]
pub enum ItemTargetPosition {
    Entity(Entity),
    Positon(GlobalTransform),
}

impl ItemTargetPosition {
    pub fn get_use_pos(
        self,
        query: &Query<(&GlobalTransform, Option<&ItemUsageOffset>)>,
    ) -> Option<Transform> {
        match self {
            ItemTargetPosition::Entity(e) => query.get(e).ok().map(|(tf, offset)| {
                let translation = tf.transform_point(offset.copied().unwrap_or_default().0);
                tf.compute_transform().with_translation(translation)
            }),
            ItemTargetPosition::Positon(tf) => Some(tf.compute_transform()),
        }
    }
}

#[derive(Component, Default, Clone, Copy, Debug)]
//local space
pub struct ItemUsageOffset(Vec3);

#[derive(Default, Clone)]
pub enum ItemAction {
    #[default]
    None,
    UsingWindup {
        elapsed_time: Stopwatch,
        target_position: ItemTargetPosition,
        sent_start_event: bool,
    },
    UsingBackswing {
        elapsed_time: Stopwatch,
    },
    //elapsed time, target position
    SwingingWindup {
        elapsed_time: Stopwatch,
        target_position: ItemTargetPosition,
        sent_start_event: bool,
    },
    SwingingBackswing {
        elapsed_time: Stopwatch,
    },
}

impl ItemAction {
    //will cancel any windup
    //doesn't affect backswing - we don't want the player to be able to switch slots to cancel it
    pub fn cancel_action(&mut self) {
        match self {
            ItemAction::None => (),
            ItemAction::UsingWindup { .. } => *self = ItemAction::None,
            ItemAction::UsingBackswing { .. } => (),
            ItemAction::SwingingWindup { .. } => *self = ItemAction::None,
            ItemAction::SwingingBackswing { .. } => (),
        }
    }
    pub fn try_swing(&mut self, tf: ItemTargetPosition) {
        if let ItemAction::None = self {
            *self = ItemAction::SwingingWindup {
                elapsed_time: Stopwatch::default(),
                target_position: tf,
                sent_start_event: false,
            }
        }
    }
    pub fn try_use(&mut self, tf: ItemTargetPosition) {
        if let ItemAction::None = self {
            *self = ItemAction::UsingWindup {
                elapsed_time: Stopwatch::default(),
                target_position: tf,
                sent_start_event: false,
            };
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
    pub fn selected_slot(&self) -> usize {
        self.selected_slot
    }
    pub fn selected_item_entity(&self) -> Option<Entity> {
        self.selected_item().map(|stack| stack.id)
    }
    pub fn selected_item(&self) -> Option<ItemStack> {
        self.get(self.selected_slot())
    }
    //if slot_num is negative or over the number of slots in the inventory, loop back around
    pub fn select_slot(&mut self, slot_num: i32) {
        //loop back around
        let new_slot = slot_num.rem_euclid(self.items.len() as i32) as usize;
        if new_slot != self.selected_slot {
            if let Some((_, action)) = &mut self.items[new_slot] {
                action.cancel_action();
            }
            if let Some((_, action)) = &mut self.items[self.selected_slot] {
                action.cancel_action();
            }
            self.selected_slot = new_slot;
        }
    }
    pub fn has_item(&self, item: Entity) -> bool {
        self.items.iter().any(|x| {
            if let Some(ref stack) = x {
                stack.0.id == item
            } else {
                false
            }
        })
    }
    pub fn number_of_type(&self, item: Entity) -> u32 {
        self.items
            .iter()
            .map(|x| {
                if let Some(ref stack) = x
                    && stack.0.id == item
                {
                    stack.0.size
                } else {
                    0
                }
            })
            .sum()
    }
    //returns what's left (if any) of the item stack after picking up
    pub fn pickup_item(
        &mut self,
        mut item: ItemStack,
        data_query: &Query<&MaxStackSize>,
    ) -> Option<ItemStack> {
        for i in 0..self.items.len() {
            let stacks = &mut self.items[i];

            match stacks {
                Some(stack) => {
                    //pick up part of the stack
                    if stack.0.id != item.id {
                        continue;
                    }
                    let picking_up = item
                        .size
                        .min(data_query.get(item.id).unwrap().0.saturating_sub(item.size));
                    if picking_up > 0 {
                        item.size -= picking_up;
                        stack.0.size += picking_up;
                    }
                }
                None => {
                    //pick up the whole stack into an empty slot
                    *stacks = Some((item, default()));
                    item.size = 0;
                    return None;
                }
            }
            if item.size == 0 {
                return None;
            }
        }
        Some(item)
    }

    pub fn can_pickup_item(&self, mut item: ItemStack, data_query: &Query<&MaxStackSize>) -> bool {
        for i in 0..self.items.len() {
            let stacks = &self.items[i];

            match stacks {
                Some(stack) => {
                    //pick up part of the stack
                    if stack.0.id != item.id {
                        continue;
                    }
                    let picking_up = item
                        .size
                        .min(data_query.get(item.id).unwrap().0.saturating_sub(item.size));
                    if picking_up > 0 {
                        item.size -= picking_up;
                    }
                }
                None => {
                    //can pick up the whole stack into an empty slot
                    return true;
                }
            }
            if item.size == 0 {
                return true;
            }
        }
        //there's some items remaining, so we can't pick it all up
        false
    }

    // returns the number of items removed
    pub fn remove_items(&mut self, item: ItemStack) -> u32 {
        let mut to_remove = item.size;
        if item.size == 0 {
            return 0;
        }
        for stack in self.iter_mut() {
            if let Some((inv_item, _)) = stack.clone()
                && inv_item.id == item.id
            {
                to_remove -= descrease_slot_size(stack, to_remove);
                if to_remove == 0 {
                    break;
                }
            }
        }
        item.size - to_remove
    }

    pub fn set_slot_no_events(&mut self, slot: usize, item: ItemStack) {
        self.items[slot] = Some((item, default()));
    }
    pub fn swap_slots(&mut self, slot_a: usize, slot_b: usize) {
        self.items.swap(slot_a, slot_b);
    }
    // returns the number of items moved
    pub fn move_items(
        &mut self,
        from_slot: usize,
        to_slot: usize,
        max_count: u32,
        data_query: &Query<&MaxStackSize>,
    ) -> u32 {
        if max_count == 0 || from_slot == to_slot {
            // no items to move
            return 0;
        }
        // block moving items if they are currently being used or on cooldown to avoid exploits
        let Some((from_stack, ItemAction::None)) = self.items[from_slot].clone() else {
            // no items to move
            return 0;
        };
        match self.items[to_slot].clone() {
            Some((to_stack, to_action)) => {
                if !matches!(to_action, ItemAction::None) || from_stack.id != to_stack.id {
                    return 0; //block move if there's an active action or the items aren't the same
                }
                let Ok(max_stack_size) = data_query.get(to_stack.id) else {
                    return 0; //invalid or non-stackable item
                };
                let moving = max_count
                    .min(from_stack.size)
                    .min(max_stack_size.0.saturating_sub(to_stack.size));
                descrease_slot_size(&mut self.items[from_slot], moving);
                self.items[to_slot].as_mut().unwrap().0.size += moving;
                moving
            }
            None => {
                let moving = max_count.min(from_stack.size);
                if moving == from_stack.size {
                    //moving all, so just swap the stacks
                    self.swap_slots(from_slot, to_slot);
                } else {
                    //we are only moving part of the stack
                    self.items[to_slot] =
                        Some((ItemStack::new(from_stack.id, moving), ItemAction::None));
                    descrease_slot_size(&mut self.items[from_slot], moving);
                }
                moving
            }
        }
    }
    //returns the dropped items
    pub fn drop_slot(&mut self, slot: usize) -> Option<ItemStack> {
        let item = self.items[slot].clone();
        self.items[slot] = None;
        let dropped = item.map(|(stack, _)| stack);
        dropped
    }
    //returns the dropped items
    pub fn drop_items(&mut self, slot: usize, max_drops: u32) -> Option<ItemStack> {
        let (stack, _) = self.items[slot].clone()?;
        descrease_slot_size(&mut self.items[slot], max_drops);
        let dropped_size = self.items[slot]
            .as_ref()
            .map(|(new_stack, _)| stack.size - new_stack.size)
            .unwrap_or(stack.size);
        let dropped_stack = ItemStack::new(stack.id, dropped_size);

        Some(dropped_stack)
    }
    pub fn use_item(&mut self, slot: usize, target: ItemTargetPosition) {
        if let Some((_, action)) = &mut self.items[slot] {
            action.try_use(target);
        }
    }
    pub fn swing_item(&mut self, slot: usize, target: ItemTargetPosition) {
        if let Some((_, action)) = &mut self.items[slot] {
            action.try_swing(target);
        }
    }
    pub fn get(&self, slot: usize) -> Option<ItemStack> {
        self.items
            .get(slot)
            .cloned()
            .flatten()
            .map(|(stack, _)| stack)
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

fn descrease_slot_size(stack_opt: &mut Option<(ItemStack, ItemAction)>, max_amount: u32) -> u32 {
    let mut removed = 0;
    *stack_opt = stack_opt.as_ref().and_then(|(stack, act)| {
        let new_size = stack.size.saturating_sub(max_amount);
        removed = stack.size - new_size;
        if new_size == 0 {
            None
        } else {
            Some((ItemStack::new(stack.id, new_size), act.clone()))
        }
    });
    removed
}

pub fn tick_item_timers(
    mut query: Query<&mut Inventory>,
    use_pos_query: Query<(&GlobalTransform, Option<&ItemUsageOffset>)>,
    swing_speed_query: Query<&ItemSwingSpeed>,
    use_speed_query: Query<&ItemUseSpeed>,
    time: Res<Time>,
    mut start_using_writer: EventWriter<StartUsingItemEvent>,
    mut use_writer: EventWriter<UseItemEvent>,
    mut start_swinging_writer: EventWriter<StartSwingingItemEvent>,
    mut swing_writer: EventWriter<SwingItemEvent>,
) {
    for mut inventory in query.iter_mut() {
        let owner = inventory.owner;
        //speeds to use if the equipped item doesn't have a speed
        let base_use_speed = use_speed_query.get(inventory.owner).ok();
        let base_swing_speed = swing_speed_query.get(inventory.owner).ok();
        for (inventory_slot, opt) in inventory.iter_mut().enumerate() {
            if let Some((stack, action)) = opt {
                match action {
                    ItemAction::None => (),
                    ItemAction::UsingWindup {
                        elapsed_time,
                        target_position,
                        sent_start_event,
                    } => {
                        elapsed_time.tick(time.delta());
                        match use_speed_query.get(stack.id).ok().fallback(base_use_speed) {
                            Some(use_speed) => {
                                if elapsed_time.elapsed() >= use_speed.windup {
                                    if let Some(tf) = target_position.get_use_pos(&use_pos_query) {
                                        use_writer.send(UseItemEvent {
                                            user: owner,
                                            inventory_slot: Some(inventory_slot),
                                            stack: *stack,
                                            tf,
                                        });
                                    } else {
                                        warn!("Invalid entity get_use_pos");
                                    }
                                    *action = ItemAction::UsingBackswing {
                                        elapsed_time: Stopwatch::default(),
                                    };
                                } else if !*sent_start_event {
                                    if let Some(tf) = target_position.get_use_pos(&use_pos_query) {
                                        start_using_writer.send(StartUsingItemEvent {
                                            user: owner,
                                            inventory_slot: Some(inventory_slot),
                                            stack: *stack,
                                            tf,
                                        });
                                    } else {
                                        warn!("Invalid entity get_use_pos");
                                    }
                                    *sent_start_event = true;
                                }
                            }
                            _ => {
                                if let Some(tf) = target_position.get_use_pos(&use_pos_query) {
                                    use_writer.send(UseItemEvent {
                                        user: owner,
                                        inventory_slot: Some(inventory_slot),
                                        stack: *stack,
                                        tf,
                                    });
                                } else {
                                    warn!("Invalid entity get_use_pos");
                                }
                                *action = ItemAction::None;
                            }
                        }
                    }
                    ItemAction::UsingBackswing { elapsed_time } => {
                        elapsed_time.tick(time.delta());
                        match use_speed_query.get(stack.id).ok().fallback(base_use_speed) {
                            Some(use_speed) => {
                                if elapsed_time.elapsed() >= use_speed.backswing {
                                    *action = ItemAction::None;
                                }
                            }
                            _ => *action = ItemAction::None,
                        }
                    }
                    ItemAction::SwingingWindup {
                        elapsed_time,
                        target_position,
                        sent_start_event,
                    } => {
                        elapsed_time.tick(time.delta());
                        match swing_speed_query
                            .get(stack.id)
                            .ok()
                            .fallback(base_swing_speed)
                        {
                            Some(swing_speed) => {
                                if elapsed_time.elapsed() >= swing_speed.windup {
                                    if let Some(tf) = target_position.get_use_pos(&use_pos_query) {
                                        swing_writer.send(SwingItemEvent {
                                            user: owner,
                                            inventory_slot: Some(inventory_slot),
                                            stack: *stack,
                                            tf,
                                        });
                                    } else {
                                        warn!("Invalid entity get_use_pos");
                                    }
                                    *action = ItemAction::SwingingBackswing {
                                        elapsed_time: Stopwatch::default(),
                                    };
                                } else if !*sent_start_event {
                                    if let Some(tf) = target_position.get_use_pos(&use_pos_query) {
                                        start_swinging_writer.send(StartSwingingItemEvent {
                                            user: owner,
                                            inventory_slot: Some(inventory_slot),
                                            stack: *stack,
                                            tf,
                                        });
                                    } else {
                                        warn!("Invalid entity get_use_pos");
                                    }
                                    *sent_start_event = true;
                                }
                            }
                            _ => {
                                if let Some(tf) = target_position.get_use_pos(&use_pos_query) {
                                    swing_writer.send(SwingItemEvent {
                                        user: owner,
                                        inventory_slot: Some(inventory_slot),
                                        stack: *stack,
                                        tf,
                                    });
                                } else {
                                    warn!("Invalid entity get_use_pos");
                                }
                                *action = ItemAction::None;
                            }
                        }
                    }
                    ItemAction::SwingingBackswing { elapsed_time } => {
                        elapsed_time.tick(time.delta());
                        match swing_speed_query
                            .get(stack.id)
                            .ok()
                            .fallback(base_swing_speed)
                        {
                            Some(swing_speed) => {
                                if elapsed_time.elapsed() >= swing_speed.backswing {
                                    *action = ItemAction::None;
                                }
                            }
                            _ => *action = ItemAction::None,
                        }
                    }
                }
            }
        }
    }
}
