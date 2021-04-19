use minecraft_format::{
    packets::{play_serverbound::ServerboundPacket},
};

pub mod dig_down;
pub mod travel;
use dig_down::DigDownMission;
use travel::TravelMission;

#[derive(Debug)]
pub enum Mission {
    None,
    DigDown(DigDownMission),
    Travel(TravelMission),
}

impl Mission {
    pub fn apply(bot: &mut crate::bot::Bot, packets: &mut Vec<ServerboundPacket>) {
        let position = match bot.position.as_mut() {
            Some(position) => position,
            None => return,
        };

        match &mut bot.mission {
            Mission::None => (),
            Mission::DigDown(mission) => {
                if mission.apply(position, &bot.map, &mut bot.windows, packets) {
                    bot.mission = Mission::None;
                }
            }
            Mission::Travel(mission) => {
                mission.apply(&mut bot.vertical_speed, &bot.map, position)
            }
        }
    }
}
