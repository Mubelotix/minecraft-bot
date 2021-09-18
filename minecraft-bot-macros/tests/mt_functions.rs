use minecraft_bot_macros::*;

pub struct Bot {}

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
fn submission(target: usize, mt_variable: usize) -> Result<String, &'static str> {
    let mut counter: usize = 0;

    'mt_finder: loop {
        if counter >= target {
            break 'mt_finder;
        }
        counter += 1;
    }

    Ok(String::from("Mission complete"))
}

#[tick_distributed]
fn mission(lorem: usize, mt_variable: usize) -> Result<usize, &'static str> {
    let message: Result<String, &'static str> = mt_submission(50);

    let mut i: u32 = 0;
    'mt_printer: loop {
        i += 1;
        println!("message: {:?}", message);

        if i > 5 {
            break 'mt_printer;
        }
    }

    Ok(5)
}

#[test]
fn test() {
    let mut mission = mission(5);

    while mission.execute(5) == MissionResult::InProgress {}
}
