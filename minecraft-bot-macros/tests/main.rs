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
    let (mut action_id, id): (u16, u8) = {
        let mut action_id = 8;
        let id = 12;
        (action_id, id)
    };

    let test: u8 = {
        let test = 25;
        test
    };

    let data: u8 = loop {
        if test == 25 {
            break;
        }
    };

    {
        println!("yeah");
    }

    
}
