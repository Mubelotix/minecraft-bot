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

#[tick_distributed]
fn test() {
    let test: u8 = 255;

    'mt_main: loop {   
        let youpi: i32 = 42;
        let yipou: i32 = 64;

        'mt_inner: loop {
            println!("A great day isn't it");
        }
    }

    {
        println!("yeah");
    }
}
