pub mod network;
pub mod bot;
use bot::Bot;

fn main() {
    env_logger::init();
    Bot::create("127.0.0.1".to_string(), 25565, "bot".to_string());
}
