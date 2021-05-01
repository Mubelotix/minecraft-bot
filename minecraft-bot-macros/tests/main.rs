use minecraft_bot_macros::*;

pub struct Bot {

}

pub enum MissionResult {
    Done,
    Failed,
    InProgress,
}

pub trait Mission {
    fn execute(&mut self, bot: &mut Bot) -> MissionResult;
}

#[fsm]
fn test() {
    let test: u8 = 255;

    loop {
        let youpi: i32 = 42;
        let yipou: i32 = 64;

        loop {
            println!("A great day isn't it");
        }
    }

    {
        println!("yeah");
    }
}
