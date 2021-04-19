use std::collections::BTreeMap;
use log::*;
use minecraft_format::{packets::Array, slots::Slot};

#[derive(Debug, Clone, Copy)]
pub struct Item {
    pub item_count: u32,
    pub item_id: minecraft_format::ids::items::Item,
}

pub struct PlayerInventory {
    slots: [Option<Item>; 46],
}

impl PlayerInventory {
    fn set_slot_clientbound(&mut self, slot_id: i16, slot_data: Slot) {
        if !(0..=45).contains(&slot_id) {
            warn!("Failed to set slot {} as there are only 46 slots in the player inventory.", slot_id);
            return;
        }
        let slot_id = slot_id as usize;
        unsafe {
            *self.slots.get_unchecked_mut(slot_id) = slot_data.item.map(|item| Item {
                item_count: std::cmp::max(item.item_count.0, 0) as u32,
                item_id: item.item_id,
            });
        }
    }

    pub fn get_crafting_output(&self) -> &Option<Item> {
        &self.slots[0]
    }

    pub fn get_crafting_input_top_left(&self) -> &Option<Item> {
        &self.slots[1]
    }

    pub fn get_crafting_input_top_right(&self) -> &Option<Item> {
        &self.slots[2]
    }

    pub fn get_crafting_input_bottom_left(&self) -> &Option<Item> {
        &self.slots[3]
    }

    pub fn get_crafting_input_bottom_right(&self) -> &Option<Item> {
        &self.slots[4]
    }

    pub fn get_helmet(&self) -> &Option<Item> {
        &self.slots[5]
    }

    pub fn get_chessplate(&self) -> &Option<Item> {
        &self.slots[6]
    }

    pub fn get_legging(&self) -> &Option<Item> {
        &self.slots[7]
    }

    pub fn get_boots(&self) -> &Option<Item> {
        &self.slots[8]
    }

    pub fn get_main_inventory(&self) -> &[Option<Item>] {
        &self.slots[9..=35]
    }

    pub fn get_hotbar(&self) -> &[Option<Item>] {
        &self.slots[36..=44]
    }

    pub fn get_offhand(&self) -> &Option<Item> {
        &self.slots[45]
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

pub struct Windows {
    pub player_inventory: PlayerInventory,
    pub cursor: Option<Item>,
    pub windows: BTreeMap<i8, Window>,
}

impl Windows {
    pub fn new() -> Self {
        Windows {
            player_inventory: PlayerInventory { slots: [None; 46] },
            cursor: None,
            windows: BTreeMap::new(),
        }
    }

    pub fn handle_open_window_packet(&mut self, window_id: i32, window_type: i32) {
        trace!("Opening window {} (type={})", window_id, window_type);
    }

    pub fn handle_update_window_items_packet<'input, 'a>(&'a mut self, window_id: i8, slots: Array<'input, minecraft_format::slots::Slot<'input>, i16>) {
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
                    match slot.item {
                        Some(item) => {
                            self.player_inventory.slots[idx] = Some(Item {
                                item_count: std::cmp::max(item.item_count.0, 0) as u32,
                                item_id: item.item_id,
                            })
                        }
                        None => {
                            self.player_inventory.slots[idx] = None;
                        }
                    }
                }
            }
            window_id => {
                trace!("todo");
            }
        }
    }

    pub fn handle_set_slot_packet(&mut self, window_id: i8, slot_id: i16, slot_data: Slot) {
        trace!("Setting slot {} in window {} to {:?}.", slot_id, window_id, slot_data);
        match window_id {
            -1 => {}
            0 => {
                self.player_inventory.set_slot_clientbound(slot_id, slot_data);
            }
            window_id => {
                trace!("todo");
            }
        }
    }

    pub fn handle_close_window_packet(&mut self, window_id: i8) {
        trace!("Closing window {}", window_id);
        if self.windows.remove(&window_id).is_none() {
            warn!("There was no window {}", window_id);
        }
    }
}
