use crate::map::Map;
use log::*;
use crate::*;
use minecraft_protocol::{
    ids::{blocks::Block, items::Item},
    packets::{play_serverbound::ServerboundPacket, serializer::MinecraftPacketPart, Array},
    components::slots::Slot,
};
use std::{collections::BTreeMap, sync::mpsc::Sender};

pub struct PlayerInventory {
    slots: [Slot; 46],
    held_item: u8,
    sender: Sender<Vec<u8>>,
}

impl PlayerInventory {
    fn set_slot_clientbound(&mut self, slot_id: i16, slot_data: Slot) {
        if !(0..=45).contains(&slot_id) {
            warn!("Failed to set slot {} as there are only 46 slots in the player inventory.", slot_id);
            return;
        }
        let slot_id = slot_id as usize;
        unsafe {
            *self.slots.get_unchecked_mut(slot_id) = slot_data;
        }
    }

    pub fn get_slots(&self) -> &[Slot; 46] {
        &self.slots
    }

    pub fn get_crafting_output(&self) -> &Slot {
        &self.slots[0]
    }

    pub fn get_crafting_input_top_left(&self) -> &Slot {
        &self.slots[1]
    }

    pub fn get_crafting_input_top_right(&self) -> &Slot {
        &self.slots[2]
    }

    pub fn get_crafting_input_bottom_left(&self) -> &Slot {
        &self.slots[3]
    }

    pub fn get_crafting_input_bottom_right(&self) -> &Slot {
        &self.slots[4]
    }

    pub fn get_helmet(&self) -> &Slot {
        &self.slots[5]
    }

    pub fn get_chessplate(&self) -> &Slot {
        &self.slots[6]
    }

    pub fn get_legging(&self) -> &Slot {
        &self.slots[7]
    }

    pub fn get_boots(&self) -> &Slot {
        &self.slots[8]
    }

    pub fn get_main_inventory(&self) -> &[Slot] {
        &self.slots[9..=35]
    }

    pub fn get_hotbar(&self) -> &[Slot] {
        &self.slots[36..=44]
    }

    pub fn get_offhand(&self) -> &Slot {
        &self.slots[45]
    }

    pub fn change_held_item(&mut self, new_held_item: u8) {
        if new_held_item >= 9 {
            panic!("Failed to change held item: {} is not a valid hotbar item id.", new_held_item);
        }
        self.held_item = new_held_item;
        self.sender
            .send(
                ServerboundPacket::HeldItemChange { slot: new_held_item as i16 }
                    .serialize_minecraft_packet()
                    .unwrap(),
            )
            .unwrap();
    }

    pub fn get_held_item(&self) -> u8 {
        self.held_item
    }

    pub fn handle_held_item_change_packet(&mut self, new_held_item: u8) {
        if new_held_item >= 9 {
            panic!(
                "Failed to change held item: {} is not a valid hotbar item id. (value is from server)",
                new_held_item
            );
        }
        self.held_item = new_held_item;
    }

    pub fn use_held_item(&mut self, uses: u8) -> bool {
        let held_item = self.slots.get_mut((36 + self.held_item) as usize).unwrap();
        let mut destroyed = false;
        if let Some(item) = held_item.item.as_mut() {
            if let Some(compound) = item.nbt_data.as_mut_compound() {
                if let Some(damage) = compound.get_mut("Damage") {
                    if let Some(damage) = damage.as_mut_int() {
                        *damage += uses as i32;
                        if let Some(durability) = item.item_id.get_durability() {
                            if *damage >= durability as i32 {
                                destroyed = true;
                            }
                        } else {
                            warn!("Item {:?} has a damage property but its durability is unknown", item);
                        }
                    } else {
                        warn!("Damage property is not a int for item {:?}", item);
                    }
                }
            }
        }
        if destroyed {
            held_item.item = None;
        }

        destroyed
    }

    pub fn place_block(&mut self, map: &mut Map, mainhand: bool, (x, y, z): (i32, i32, i32)) -> Result<(), ()> {
        let item_id = match mainhand {
            true => 36 + self.held_item,
            false => 45,
        };

        let mut slot = self.slots.get_mut(item_id as usize);
        let slot = slot.as_mut().unwrap();
        let item = &mut slot.item;

        if item.is_none() {
            warn!("Cannot place block: No item in the selected hand");
            return Err(());
        }
        let block = match item.as_ref().unwrap().item_id {
            Item::Andesite => Block::Andesite,
            Item::Cobblestone => Block::Cobblestone,
            Item::Granite => Block::Granite,
            Item::Dirt => Block::Dirt,
            item => {
                warn!("Unknown item {:?} after block placement. Using Stone for compatibility", item);
                Block::Stone
            }
        };

        match item.as_ref().unwrap().item_count.0 {
            item_count if item_count <= 0 => {
                warn!("Cannot place block: No item left in this slot");
                return Err(());
            }
            item_count if item_count == 1 => {
                *item = None;
            }
            _item_count => {
                item.as_mut().unwrap().item_count.0 -= 1;
            }
        }

        use minecraft_protocol::{components::blocks::BlockFace, components::slots::Hand};
        self.sender
            .send(
                ServerboundPacket::PlaceBlock {
                    hand: if mainhand { Hand::MainHand } else { Hand::OffHand },
                    location: Position { x, y: y as i16, z },
                    face: BlockFace::Top,
                    cursor_position_x: 0.5,
                    cursor_position_y: 0.5,
                    cursor_position_z: 0.5,
                    inside_block: false,
                }
                .serialize_minecraft_packet()
                .unwrap(),
            )
            .unwrap();

        map.set_block(x, y, z, block);

        Ok(())
    }
}

impl std::fmt::Debug for PlayerInventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlayerInventory {{ ")?;
        for (idx, slot) in self.slots.iter().enumerate() {
            if let Some(item) = &slot.item {
                write!(f, "{}: {} {:?}, ", idx, item.item_count.0, item.item_id)?;
            }
        }
        write!(f, "}}")?;
        Ok(())
    }
}

pub enum Window {
    Chest,
    LargeChest,
    CraftingTable,
    Furnace,
    BlastFurnace,
    Smoker,
    Dispenser,
    EnchantmentTable,
    BrewingStand,
    VillagerTrading,
    Beacon,
    Anvil,
    Hopper,
    ShulkerBox,
    Llama,
    Horse,
    Donkey,
    CartographyTable,
    Grindstone,
    Lectern,
    Loom,
    Stonecutter,
}

use array_macro::array;

pub struct Windows {
    pub player_inventory: PlayerInventory,
    cursor: Slot,
    pub windows: BTreeMap<i8, Window>,
    window_click_states: BTreeMap<i8, Vec<Option<bool>>>,
    sender: Sender<Vec<u8>>,
}

impl Windows {
    pub fn new(sender: Sender<Vec<u8>>) -> Self {
        let mut window_click_states = BTreeMap::new();
        window_click_states.insert(0, Vec::new());

        Windows {
            player_inventory: PlayerInventory {
                slots: array![Slot {item: None}; 46],
                sender: sender.clone(),
                held_item: 0,
            },
            cursor: Slot { item: None },
            windows: BTreeMap::new(),
            window_click_states,
            sender,
        }
    }

    pub fn get_cursor(&self) -> &Slot {
        &self.cursor
    }

    pub fn click_slot(&mut self, window_id: i8, slot_id: usize) -> i16 {
        let item = match window_id {
            0 => {
                /*trace!(
                    "clicking item at {}: cursor = {:?}, clicked = {:?}",
                    slot_id,
                    self.cursor,
                    self.player_inventory.slots[slot_id]
                );*/

                // It might be possible to make an addition
                let mut addition_result = None;
                if let (Some(cursor_item), Some(clicked_item)) = (self.cursor.item.as_mut(), self.player_inventory.slots[slot_id].item.as_mut()) {
                    if cursor_item.item_id == clicked_item.item_id && cursor_item.nbt_data.is_null() && clicked_item.nbt_data.is_null() {
                        let clicked_item = clicked_item.clone();
                        let cursor_item = self.cursor.item.take().unwrap();
                        let mut target_item = self.player_inventory.slots[slot_id].item.as_mut().unwrap();
                        target_item.item_count.0 += cursor_item.item_count.0;
                        let max_stack_size = target_item.item_id.get_max_stack_size() as i32;
                        if target_item.item_count.0 > max_stack_size {
                            self.cursor.item = Some(minecraft_protocol::components::slots::SlotItem {
                                item_id: target_item.item_id,
                                item_count: minecraft_protocol::packets::VarInt(target_item.item_count.0 - max_stack_size),
                                nbt_data: minecraft_protocol::nbt::NbtTag::Null,
                            });
                            target_item.item_count.0 = max_stack_size;
                        }
                        addition_result = Some(clicked_item);
                    }
                }

                // Otherwise we swap the stacks
                match addition_result {
                    Some(result) => Some(result),
                    None => {
                        let clicked_item = self.player_inventory.slots[slot_id].item.take();
                        self.player_inventory.slots[slot_id].item = self.cursor.item.take();
                        self.cursor.item = clicked_item.clone();
                        clicked_item
                    }
                }
            }
            window_id => {
                todo!()
            }
        };
        let action_id = self.register_new_action(window_id);
        self.sender
            .send(
                ServerboundPacket::ClickWindowSlot {
                    window_id,
                    slot: slot_id as i16,
                    button: 0,
                    action_id,
                    mode: 0.into(),
                    clicked_item: Slot { item },
                }
                .serialize_minecraft_packet()
                .unwrap(),
            )
            .unwrap();
        action_id
    }

    fn register_new_action(&mut self, window_id: i8) -> i16 {
        if let Some(window_click_states) = self.window_click_states.get_mut(&window_id) {
            window_click_states.push(None);
            (window_click_states.len() - 1) as i16
        } else {
            error!("New action registered in closed window");
            0
        }
    }

    pub fn get_action_state(&self, window_id: i8, action_id: i16) -> &Option<bool> {
        if let Some(window_click_states) = self.window_click_states.get(&window_id) {
            if let Some(state) = window_click_states.get(action_id as usize) {
                state
            } else {
                warn!("Window action state with inexistant id ({})", action_id);
                &None
            }
        } else {
            warn!("Window action state requested in closed window");
            &None
        }
    }

    pub fn handle_open_window_packet(&mut self, window_id: i32, window_type: i32) {
        self.window_click_states.insert(window_id as i8, Vec::new());
        trace!("Opening window {} (type={})", window_id, window_type);
    }

    pub fn handle_update_window_items_packet(&mut self, window_id: i8, slots: Array<minecraft_protocol::components::slots::Slot, i16>) {
        trace!("Updating window {} ({} items)", window_id, slots.items.len());
        match window_id {
            0 => {
                if slots.items.len() != 46 {
                    error!(
                        "Failed to update window items. Player inventory contains 46 slots but {} where received.",
                        slots.items.len()
                    );
                    return;
                }
                for (idx, slot) in slots.items.into_iter().enumerate() {
                    self.player_inventory.slots[idx] = slot;
                }

                trace!("Inventory: {:?}", self.player_inventory);
            }
            window_id => {
                trace!("todo");
            }
        }
    }

    pub fn handle_set_slot_packet(&mut self, window_id: i8, slot_id: i16, slot_data: Slot) {
        trace!("Setting slot {} in window {} to {:?}.", slot_id, window_id, slot_data);
        match window_id {
            -1 if slot_id == -1 => {
                self.cursor = slot_data;
            }
            0 => {
                self.player_inventory.set_slot_clientbound(slot_id, slot_data);
            }
            window_id => {
                trace!("todo");
            }
        }
    }

    pub fn handle_window_confirmation_packet(&mut self, window_id: i8, action_id: i16, accepted: bool) {
        if let Some(window_click_states) = self.window_click_states.get_mut(&window_id) {
            if let Some(state) = window_click_states.get_mut(action_id as usize) {
                *state = Some(accepted)
            } else {
                warn!("Window confirmation received with inexistant action_id: {} in {}", action_id, window_id);
            }
        }
    }

    pub fn handle_close_window_packet(&mut self, window_id: i8) {
        trace!("Closing window {}", window_id);
        let r1 = self.windows.remove(&window_id).is_none();
        let r2 = self.window_click_states.remove(&window_id).is_none();
        if r1 || r2 {
            warn!("There was no window {} ({}, {})", window_id, r1, r2);
        }
    }
}
