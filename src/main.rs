#![allow(clippy::new_without_default)]

pub mod bot;
pub mod network;
pub mod map;
use bot::Bot;

fn main() {
    env_logger::init();
    Bot::create("127.0.0.1".to_string(), 25565, "bot".to_string());
}
