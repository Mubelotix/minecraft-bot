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
fn test() -> Result<(), &'static str> {
    let (action_id, id): (u16, u8) = {
        
    };

    let (test1, test2): (String, String) = {
        
    };

    let (t8): (usize) = {
        
    };

    let t9: u128 = {
        
    };

    {
        
    }

    let t10: (u128) = {
        println!("Test1 = {}", test1);
    };

    let t11: String = {

    };
}
