use minecraft_format::{
    packets::{play_serverbound::ServerboundPacket},
};

pub mod dig_down;
pub mod travel;
pub mod inventory;
pub use inventory::*;
pub use dig_down::DigDownMission;
pub use travel::TravelMission;

use crate::bot::Bot;

pub trait Mission: Send {
    fn execute(&mut self, bot: &mut Bot, packets: &mut Vec<ServerboundPacket>) -> bool;
}
