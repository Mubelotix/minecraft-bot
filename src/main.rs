#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::result_unit_err)]

pub mod bot;
pub mod map;
pub mod network;
pub mod pathfinder;
pub mod inventory;
pub mod missions;
use bot::Bot;

fn main() {
    env_logger::init();
    Bot::create("127.0.0.1".to_string(), 25565, "bot".to_string());
}
