use crate::*;

mod dig_down;
mod travel;
pub use dig_down::*;
pub use travel::*;

#[derive(Debug, PartialEq)]
pub enum MissionResult<T> {
    InProgress,
    Done(T),
    Outdated,
}

pub trait Mission<T>: Send {
    fn execute(&mut self, bot: &mut Bot, packets: &mut Vec<ServerboundPacket>) -> MissionResult<T>;
}
