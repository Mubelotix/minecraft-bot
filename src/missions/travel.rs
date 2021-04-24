use minecraft_format::packets::play_serverbound::ServerboundPacket;

use crate::{bot::Bot, map::Map, pathfinder::Path};

use super::MissionResult;

#[derive(Debug)]
pub struct TravelMission {
    path: Path,
}

impl TravelMission {
    pub fn new(map: &Map, position: (i32, i32, i32), destination: (i32, i32, i32)) -> Option<Self> {
        Some(Self {
            path: Path::find_path(map, position, destination)?
        })
    }
}

impl super::Mission for TravelMission {
    fn execute(&mut self, bot: &mut Bot, _packets: &mut Vec<ServerboundPacket>) -> MissionResult {
        if let Some(position) = bot.position.as_mut() {
            if let Some(((x, z), jump)) = self.path.follow((position.x, position.y, position.z), &bot.map) {
                position.x = x;
                position.z = z;
                if jump {
                    bot.vertical_speed = 0.4;
                }
            }
        }
        MissionResult::InProgress
    }
}