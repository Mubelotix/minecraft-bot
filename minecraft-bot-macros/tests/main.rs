use minecraft_bot_macros::*;

pub struct Bot {

}

pub enum MissionResult<T> {
    InProgress,
    Done(T),
    Outdated,
}

pub trait Mission<T> {
    fn execute(&mut self, bot: &mut Bot) -> MissionResult<T>;
}

#[tick_distributed]
fn test(lorem: String, ipsum: u16, dolor: u8) -> Result<usize, &'static str> {
    let test: u8 = 255;
    let mut youpi: i32 = 42;
    let mut yipou: i32 = 64;

    'mt_main: loop {   
        youpi += 1;
        let test2: i32 = 5;
        let test3: u64 = 5;
        if youpi < 60 {
            continue 'mt_main;
        }

        'mt_inner: loop {
            yipou += 1;
            if yipou > 120 {
                break 'mt_main;
            }
        }
    }

    {
        println!("yeah");
    }

    let mut x: u8 = 7;
    x = 5;
    Ok(42)
}
