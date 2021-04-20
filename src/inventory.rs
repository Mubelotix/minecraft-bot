use std::{collections::BTreeMap, sync::mpsc::Sender};
use log::*;
use minecraft_format::{packets::{Array, serializer::MinecraftPacketPart, play_serverbound::ServerboundPacket}, slots::Slot};

pub struct PlayerInventory {
    slots: [Slot; 46],
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
    pub cursor: Slot,
    pub windows: BTreeMap<i8, (Window, Vec<Option<bool>>)>,
    sender: Sender<Vec<u8>>,
}

impl Windows {
    pub fn new(sender: Sender<Vec<u8>>) -> Self {
        Windows {
            player_inventory: PlayerInventory { slots: array![Slot {item: None}; 46] },
            cursor: Slot{item: None},
            windows: BTreeMap::new(),
            sender,
        }
    }

    pub fn click_slot(&mut self, window_id: i8, slot_id: usize) -> i16 {
        let item = match window_id {
            0 => {
                let clicked_item = self.player_inventory.slots[slot_id].item.take();
                self.player_inventory.slots[slot_id].item = self.cursor.item.take();
                self.cursor.item = clicked_item.clone();
                clicked_item
            }
            window_id => {
                todo!()
            }
        };
        let action_id = self.register_new_action(window_id);
        self.sender.send(ServerboundPacket::ClickWindowSlot {
            window_id,
            slot: slot_id as i16,
            button: 0,
            action_id,
            mode: 0.into(),
            clicked_item: Slot {item},
        }.serialize_minecraft_packet().unwrap()).unwrap();
        action_id
    }

    fn register_new_action(&mut self, window_id: i8) -> i16 {
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.1.push(None);
            (window.1.len() - 1) as i16
        } else {
            error!("New action registered in closed window");
            0
        }
    }

    pub fn get_action_state(&self, window_id: i8, action_id: i16) -> &Option<bool> {
        if let Some(window) = self.windows.get(&window_id) {
            if let Some(state) = window.1.get(action_id as usize) {
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
        trace!("Opening window {} (type={})", window_id, window_type);
    }

    pub fn handle_update_window_items_packet(&mut self, window_id: i8, slots: Array<minecraft_format::slots::Slot, i16>) {
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

    pub fn handle_window_confirmation_packet(&mut self, window_id: i8, action_id: i16, accepted: bool) {
        if let Some(window) = self.windows.get_mut(&window_id) {
            if let Some(state) = window.1.get_mut(action_id as usize) {
                *state = Some(accepted)
            } else {
                warn!("Window confirmation received with inexistant action_id: {} in {}", action_id, window_id);
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
