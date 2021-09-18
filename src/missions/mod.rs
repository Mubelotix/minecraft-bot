use crate::*;

mod dig_down;
pub use dig_down::*;

#[derive(Debug, PartialEq)]
pub enum MissionResult<T> {
    InProgress,
    Done(T),
    Outdated,
}

pub trait Mission<T>: Send {
    fn execute(&mut self, bot: &mut Bot, packets: &mut Vec<ServerboundPacket>) -> MissionResult<T>;
}
