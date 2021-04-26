use minecraft_format::packets::play_serverbound::ServerboundPacket;

pub mod dig_down;
pub mod inventory;
pub mod settlement;
pub mod travel;
pub use dig_down::DigDownMission;
pub use inventory::*;
pub use settlement::*;
pub use travel::TravelMission;

use crate::bot::Bot;

pub trait Mission: Send {
    fn execute(&mut self, bot: &mut Bot, packets: &mut Vec<ServerboundPacket>) -> MissionResult;
}

#[derive(Debug, Clone, Copy)]
pub enum MissionResult {
    InProgress,
    Done,
    Failed,
}
