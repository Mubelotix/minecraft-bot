use minecraft_bot_macros::*;

#[derive(Debug, PartialEq)]
pub enum MissionResult<T> {
    InProgress,
    Done(T),
    Outdated,
}

pub trait Mission<T> {
    fn execute(&mut self, variable: usize) -> MissionResult<T>;
}

#[tick_distributed]
fn mission(lorem: String, ipsum: u16, dolor: u8, mt_variable: usize) -> Result<usize, &'static str> {
    let init: u8 = 255;

    let final_value: usize = 'mt_loopy_the_loop: loop {
        let mut counter: usize = 0;
        'mt_inner: loop {
            counter += 1;
            if counter > 50 {
                break 'mt_loopy_the_loop 42;
            }
        }
    };

    Ok(final_value)
}

#[test]
fn test() {
    let mut mission = mission("lorem".to_string(), 5, 5);

    let mut i = 0;
    while mission.execute(i) == MissionResult::InProgress {
        i += 1;
    }
}
