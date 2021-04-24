use crate::*;
use minecraft_format::packets::play_serverbound::ServerboundPacket;
use std::cmp::Ordering;
use super::MissionResult;

#[derive(Debug)]
pub struct TravelMission {
    path: Vec<(i32, i32, i32)>,
    destination: (i32,i32,i32),
    stucked_detector: usize,
    failed: bool,
}

impl TravelMission {
    pub fn new(map: &Map, position: (i32, i32, i32), destination: (i32, i32, i32)) -> Option<Self> {
        Some(Self {
            path: find_path(map, position, destination)?,
            destination,
            stucked_detector: 0,
            failed: false,
        })
    }
}

impl super::Mission for TravelMission {
    fn execute(&mut self, bot: &mut Bot, _packets: &mut Vec<ServerboundPacket>) -> MissionResult {
        if self.failed {
            return MissionResult::Failed;
        }

        if let Some(position) = bot.position.as_mut() {
            let ((x, z), jump) = {
                let mut position = (position.x, position.y, position.z);
                let x = position.0.floor() as i32;
                let y = position.1.floor() as i32;
                let z = position.2.floor() as i32;
                let mut jump = false;

                if self.stucked_detector > 100 {
                    warn!("Bot is stucked while traveling. Recalculating...");
                    let new_path = match find_path(&bot.map, (x, y, z), self.destination) {
                        Some(new_path) => new_path,
                        None => {
                            self.failed = true;
                            warn!("Failed to find a new path");
                            return MissionResult::Failed;
                        }
                    };
                    self.path = new_path;
                    self.stucked_detector = 0;
                    return MissionResult::InProgress;
                }

                let next_position = match self.path.get(0) {
                    Some(next) => next,
                    None => return MissionResult::Done,
                };
                if next_position.0 == x && next_position.2 == z && (y - 2..=y).contains(&next_position.1) {
                    self.path.remove(0);
                    self.stucked_detector = 0;
                    return MissionResult::InProgress;
                }

                if next_position.1 > y && bot.map.is_on_ground(position.0, position.1, position.2) {
                    jump = true;
                }
                let mut movement_required = (next_position.0 as f64 + 0.5 - position.0).abs();
                if movement_required > 0.2 {
                    movement_required = 0.2;
                }

                match (next_position.0 as f64 + 0.5).partial_cmp(&position.0) {
                    Some(Ordering::Less) => {
                        let max = bot.map.max_west_movement(position.0, position.1, position.2);
                        position.0 -= if max > movement_required { movement_required } else { max };
                    }
                    Some(Ordering::Greater) => {
                        let max = bot.map.max_east_movement(position.0, position.1, position.2);
                        position.0 += if max > movement_required { movement_required } else { max };
                    }
                    _ => {}
                }

                let mut movement_required = (next_position.2 as f64 + 0.5 - position.2).abs();
                if movement_required > 0.2 {
                    movement_required = 0.2;
                }
                match (next_position.2 as f64 + 0.5).partial_cmp(&position.2) {
                    Some(Ordering::Less) => {
                        let max = bot.map.max_north_movement(position.0, position.1, position.2);
                        position.2 -= if max > movement_required { movement_required } else { max };
                    }
                    Some(Ordering::Greater) => {
                        let max = bot.map.max_south_movement(position.0, position.1, position.2);
                        position.2 += if max > movement_required { movement_required } else { max };
                    }
                    _ => {}
                }

                self.stucked_detector += 1;
                ((position.0, position.2), jump)
            };

            position.x = x;
            position.z = z;
            if jump {
                bot.vertical_speed = 0.4;
            }
        }
        MissionResult::InProgress
    }
}
