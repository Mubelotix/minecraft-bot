use crate::*;

mod dig_down;
mod travel;
mod wood_cutting;
pub use dig_down::*;
pub use travel::*;
pub use wood_cutting::*;

#[derive(Debug, PartialEq)]
pub enum MissionResult<T> {
    InProgress,
    Done(T),
    Outdated,
}

pub trait Mission<T>: Send {
    fn execute(&mut self, bot: &mut Bot, packets: &mut Vec<ServerboundPacket>) -> MissionResult<T>;
}
