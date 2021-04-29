use super::*;
use crate::*;

enum CraftCraftTableState {
    Done,
    Failed,
}
use CraftCraftTableState::*;

pub struct CraftCraftTable {
    state: CraftCraftTableState,
}

impl Mission for CraftCraftTable {
    fn execute(&mut self, bot: &mut Bot, packets: &mut Vec<ServerboundPacket>) -> MissionResult {
        match &mut self.state {


            Done => {
                return MissionResult::Done;
            }
            Failed => {
                return MissionResult::Failed;
            }
        }
        MissionResult::InProgress
    }
}
