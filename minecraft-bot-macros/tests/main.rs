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
    let mut youpi: i32 = 42;
    let mut yipou: i32 = 64;

    'mt_main: loop {   
        youpi += 1;
        if youpi < 60 {
            continue 'mt_main;
        }

        'mt_inner: loop {
            yipou += 1;
            if yipou > 120 {
                break 'mt_main;
            }
        }

        println!("t");
    }

    {
        println!("yeah");
    }

    let mut x: u8 = 7;
    x = 5;
}
