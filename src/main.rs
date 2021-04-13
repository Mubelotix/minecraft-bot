pub mod network;
pub mod bot;
use bot::Bot;

fn main() {
    Bot::create("127.0.0.1", 25565, "bot");
}
