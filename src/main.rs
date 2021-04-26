#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::result_unit_err)]

pub mod bot;
pub mod entities;
pub mod inventory;
pub mod map;
pub mod missions;
pub mod network;
pub mod pathfinder;
use bot::Bot;

pub use log::*;
pub use minecraft_format::{
    ids::{blocks::Block, entities::Entity, items::Item},
    packets::{play_clientbound::ClientboundPacket, play_serverbound::ServerboundPacket, Position, VarInt},
    slots::Slot,
};
pub use {bot::*, entities::*, inventory::*, map::*, missions::*, pathfinder::*};

fn main() {
    env_logger::init();
    Bot::create("127.0.0.1".to_string(), 25565, "bot".to_string());
}
