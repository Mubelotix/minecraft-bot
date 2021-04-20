use crate::missions::Mission;
use log::*;
use minecraft_format::{ids::items::Item, packets::play_serverbound::ServerboundPacket};

pub struct MoveItemToHotbar {
    minimum: i32,
    potential_items: Vec<Item>,
    hotbar_slot: Option<usize>,
    state: MoveItemState,
}

impl MoveItemToHotbar {
    pub fn new(minimum: u32, potential_items: Vec<Item>, hotbar_slot: Option<usize>) -> Self {
        Self {
            minimum: minimum as i32,
            potential_items,
            hotbar_slot,
            state: MoveItemState::PickItem,
        }
    }
}
enum MoveItemState {
    PickItem,
    WaitPickConfirmation { action_id: i16 },
    PutItem,
    WaitPutConfirmation { action_id: i16 },
    SweepCursor,
    WaitSweepConfirmation { action_id: i16 },
    Done,
}

impl Mission for MoveItemToHotbar {
    fn execute(&mut self, bot: &mut crate::bot::Bot, _packets: &mut Vec<ServerboundPacket>) -> bool {
        match self.state {
            MoveItemState::PickItem => {
                // Find item
                let mut slot_idx = None;
                for (idx, slot) in bot.windows.player_inventory.get_main_inventory().iter().enumerate() {
                    if let Some(item) = &slot.item {
                        if self.potential_items.contains(&item.item_id) && item.item_count.0 >= self.minimum {
                            slot_idx  = Some(idx);
                        }
                    }
                }
                let slot_id = match slot_idx {
                    Some(slot_idx) => slot_idx + 9,
                    None => {
                        warn!("Could not find item");
                        return false;
                    }
                };

                // Click item
                let action_id = bot.windows.click_slot(0, slot_id);
                self.state = MoveItemState::WaitPickConfirmation {action_id};
            }
            MoveItemState::WaitPickConfirmation { action_id } => {
                match bot.windows.get_action_state(0, action_id) {
                    Some(true) => {
                        self.state = MoveItemState::PutItem;
                    }
                    Some(false) => {
                        warn!("Denied item move (pick)");
                        self.state = MoveItemState::PickItem;
                    }
                    None => {}
                }
            }
            MoveItemState::PutItem => {
                let destination_slot_id = match self.hotbar_slot {
                    Some(hotbar_slot_id) => hotbar_slot_id + 9,
                    None => 45, // offhand id
                };

                // Click item
                let action_id = bot.windows.click_slot(0, destination_slot_id);
                self.state = MoveItemState::WaitPutConfirmation {action_id};
            }
            MoveItemState::WaitPutConfirmation { action_id } => {
                match bot.windows.get_action_state(0, action_id) {
                    Some(true) => {
                        self.state = MoveItemState::SweepCursor;
                    }
                    Some(false) => {
                        warn!("Denied item move (put)");
                        self.state = MoveItemState::PutItem;
                    }
                    None => {}
                }
            }
            MoveItemState::SweepCursor => {
                let cursor_item = match bot.windows.cursor.item.take() {
                    Some(item) => item,
                    None => {
                        self.state = MoveItemState::Done;
                        return true;
                    },
                };

                // Find empty slot in inventory
                let mut empty_slot_idx = None;
                for (idx, slot) in bot.windows.player_inventory.get_main_inventory().iter().enumerate() {
                    if slot.item.is_none() {
                        empty_slot_idx = Some(idx);
                    }
                }
                let empty_slot_id = match empty_slot_idx {
                    Some(empty_slot_idx) => empty_slot_idx + 9,
                    None => {
                        warn!("Could not find an empty slot to sweep cursor item");
                        bot.windows.cursor.item = Some(cursor_item);
                        return true;
                    }
                };

                // Click empty slot
                let action_id = bot.windows.click_slot(0, empty_slot_id);
                self.state = MoveItemState::WaitSweepConfirmation {action_id};
            }
            MoveItemState::WaitSweepConfirmation { action_id } => {
                match bot.windows.get_action_state(0, action_id) {
                    Some(true) => {
                        self.state = MoveItemState::Done;
                        return true;
                    }
                    Some(false) => {
                        warn!("Denied item move (put)");
                        self.state = MoveItemState::PutItem;
                    }
                    None => {}
                }
            }
            MoveItemState::Done => {
                return true;
            }
        }
        false
    }
}
