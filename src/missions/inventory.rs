use super::MissionResult;
use crate::missions::Mission;
use log::*;
use minecraft_protocol::{ids::items::Item, packets::play_serverbound::ServerboundPacket};

#[derive(Debug)]
pub struct MoveItemToHotbar {
    minimum: i32,
    items: Vec<Item>,
    hotbar_slot: Option<usize>,
    state: MoveItemState,
}

impl MoveItemToHotbar {
    pub fn new(minimum: u32, items: Vec<Item>, hotbar_slot: Option<usize>) -> Self {
        Self {
            minimum: minimum as i32,
            items,
            hotbar_slot,
            state: MoveItemState::PickItem,
        }
    }
}

#[derive(Debug)]
enum MoveItemState {
    CheckNeed,
    PickItem,
    WaitPickConfirmation { action_id: i16 },
    PutItem,
    WaitPutConfirmation { action_id: i16 },
    SweepCursor,
    WaitSweepConfirmation { action_id: i16 },

    Failed,
    Done,
}

impl MoveItemState {
    fn fail(&mut self, msg: &str) -> MissionResult {
        *self = MoveItemState::Failed;
        error!("Failed mission: {}", msg);
        MissionResult::Failed
    }
}

impl Mission for MoveItemToHotbar {
    fn execute(&mut self, bot: &mut crate::bot::Bot, _packets: &mut Vec<ServerboundPacket>) -> MissionResult {
        match self.state {
            MoveItemState::CheckNeed => {
                let destination_slot_id = match self.hotbar_slot {
                    Some(hotbar_slot_id) => hotbar_slot_id + 36,
                    None => 45, // offhand id
                };

                if let Some(item) = &bot.windows.player_inventory.get_slots()[destination_slot_id].item {
                    if self.items.contains(&item.item_id) && item.item_count.0 >= self.minimum {
                        trace!(
                            "Mission complete: Moved at least {} items (one of {:?}) to hotbar slot {:?}",
                            self.minimum,
                            self.items,
                            self.hotbar_slot
                        );
                        self.state = MoveItemState::Done;
                        return MissionResult::Done;
                    }
                }
                self.state = MoveItemState::PickItem;
            }
            MoveItemState::PickItem => {
                // Find item
                let mut slot_id = None;
                for asked_item in &self.items {
                    for (idx, slot) in bot.windows.player_inventory.get_slots().iter().enumerate() {
                        if let Some(item) = &slot.item {
                            // TODO gather items that are on multiple slots
                            if *asked_item == item.item_id && item.item_count.0 >= self.minimum {
                                slot_id = Some(idx);
                                break;
                            }
                        }
                    }
                }
                let slot_id = match slot_id {
                    Some(slot_id) => slot_id,
                    None => {
                        return self
                            .state
                            .fail(&format!("Could not find more than {} items (any of {:?}).", self.minimum, self.items))
                    }
                };

                // Click item
                let action_id = bot.windows.click_slot(0, slot_id);
                self.state = MoveItemState::WaitPickConfirmation { action_id };
            }
            MoveItemState::WaitPickConfirmation { action_id } => match bot.windows.get_action_state(0, action_id) {
                Some(true) => {
                    self.state = MoveItemState::PutItem;
                }
                Some(false) => {
                    self.state = MoveItemState::PickItem;
                }
                None => {}
            },
            MoveItemState::PutItem => {
                let destination_slot_id = match self.hotbar_slot {
                    Some(hotbar_slot_id) => hotbar_slot_id + 36,
                    None => 45, // offhand id
                };

                // Click item
                let action_id = bot.windows.click_slot(0, destination_slot_id);
                self.state = MoveItemState::WaitPutConfirmation { action_id };
            }
            MoveItemState::WaitPutConfirmation { action_id } => match bot.windows.get_action_state(0, action_id) {
                Some(true) => {
                    self.state = MoveItemState::SweepCursor;
                }
                Some(false) => {
                    self.state = MoveItemState::PutItem;
                }
                None => {}
            },
            MoveItemState::SweepCursor => {
                if bot.windows.get_cursor().item.is_none() {
                    return {
                        self.state = MoveItemState::CheckNeed;
                        MissionResult::InProgress
                    };
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
                        return {
                            self.state = MoveItemState::CheckNeed;
                            MissionResult::InProgress
                        };
                    }
                };

                // Click empty slot
                let action_id = bot.windows.click_slot(0, empty_slot_id);
                self.state = MoveItemState::WaitSweepConfirmation { action_id };
            }
            MoveItemState::WaitSweepConfirmation { action_id } => match bot.windows.get_action_state(0, action_id) {
                Some(true) => {
                    return {
                        self.state = MoveItemState::CheckNeed;
                        MissionResult::InProgress
                    }
                }
                Some(false) => {
                    self.state = MoveItemState::SweepCursor;
                }
                None => {}
            },
            MoveItemState::Done => {
                return MissionResult::Done;
            }
            MoveItemState::Failed => {
                return MissionResult::Failed;
            }
        }

        MissionResult::InProgress
    }
}
