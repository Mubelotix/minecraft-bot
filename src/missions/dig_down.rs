use super::*;
use crate::bot::Bot;
use log::*;
use minecraft_format::{
    ids::{blocks::Block, items::Item},
    packets::{play_serverbound::ServerboundPacket, Position},
};

#[derive(Debug)]
pub struct DigDownMission {
    until_block: usize,
    state: DigDownState,
}

impl DigDownMission {
    pub fn new(until_block: usize) -> Self {
        Self {
            until_block,
            state: DigDownState::MoveToBlockCenter,
        }
    }
}

#[derive(Debug)]
enum DigDownState {
    MoveToBlockCenter,
    FindTool {submission: MoveItemToHotbar},
    FindBlocks {submission: MoveItemToHotbar},
    StartDigging,
    WaitDigging { ticks: usize },
    FinishDigging,

    Done,
    Failed,
}

impl DigDownState {
    fn fail(&mut self, msg: &str) -> MissionResult {
        *self = DigDownState::Failed;
        error!("Failed mission: {}", msg);
        MissionResult::Failed
    }

    fn complete(&mut self, msg: &str) -> MissionResult {
        *self = DigDownState::Done;
        debug!("Mission complete: {}", msg);
        MissionResult::Done
    }
}

impl super::Mission for DigDownMission {
    fn execute(&mut self, bot: &mut Bot, packets: &mut Vec<ServerboundPacket>) -> MissionResult {
        let position = match bot.position.as_mut() {
            Some(position) => position,
            None => return self.state.fail("Cannot dig down if the position is unknown"),
        };

        match &mut self.state {
            DigDownState::MoveToBlockCenter => {
                let mut offset_x = 0.5 - (position.x - position.x.floor());
                let mut offset_z = 0.5 - (position.z - position.z.floor());
                let mut done = true;
                if offset_x > 0.2 {
                    offset_x = 0.2;
                    done = false;
                } else if offset_x < -0.2 {
                    offset_x = -0.2;
                    done = false;
                }
                if offset_z > 0.2 {
                    offset_z = 0.2;
                    done = false;
                } else if offset_z < -0.2 {
                    offset_z = -0.2;
                    done = false;
                }

                position.x += offset_x;
                position.z += offset_z;

                packets.push(ServerboundPacket::HeldItemChange{slot: 0});

                if done {
                    self.state = DigDownState::FindTool {submission: MoveItemToHotbar::new(1, vec![Item::IronPickaxe, Item::StonePickaxe, Item::WoodenPickaxe], Some(0)) };
                }
            }
            DigDownState::FindTool {submission} => {
                match submission.execute(bot, packets) {
                    MissionResult::Done | MissionResult::Failed => self.state = DigDownState::FindBlocks{submission: MoveItemToHotbar::new(5, vec![Item::Andesite, Item::Granite, Item::Stone, Item::Dirt], None) },
                    MissionResult::InProgress => (),
                }
            }
            DigDownState::FindBlocks {submission} => {
                match submission.execute(bot, packets) {
                    MissionResult::Done => self.state = DigDownState::StartDigging,
                    MissionResult::Failed => self.state = {
                        warn!("Could not find blocks");
                        DigDownState::StartDigging
                    },
                    MissionResult::InProgress => (),
                }
            },
            DigDownState::StartDigging => {
                if position.y.floor() as isize <= self.until_block as isize {
                    return self
                        .state
                        .complete(&format!("Mission complete: Made a hole deeper than {}", self.until_block));
                }
                let (x, y, z) = (position.x.floor() as i32, position.y.floor() as i32 - 1, position.z.floor() as i32);
                let block = bot.map.get_block(x, y, z);
                if !block.is_diggable() {
                    return self.state.fail(&format!("Failed to dig, block {:?} is not diggable", block));
                }
                let compatible_harvest_tools = block.get_compatible_harvest_tools();
                let (can_harvest, speed_multiplier) = match &bot.windows.player_inventory.get_hotbar()[0].item {
                    Some(tool) => (
                        compatible_harvest_tools.is_empty() || compatible_harvest_tools.contains(&(tool.item_id as u32)),
                        match tool.item_id {
                            Item::WoodenPickaxe => 2,
                            Item::StonePickaxe => 2,
                            Item::IronPickaxe => 6,
                            Item::DiamondPickaxe => 8,
                            Item::NetheritePickaxe => 9,
                            Item::GoldenPickaxe => 12,
                            _ => 1,
                        },
                    ),
                    None => (compatible_harvest_tools.is_empty(), 1),
                };
                let mut time_required = block.get_hardness() as f64;
                match can_harvest {
                    true => {
                        time_required *= 1.5;
                        time_required /= speed_multiplier as f64;
                    }
                    false => time_required *= 5.0,
                }
                let ticks = (time_required * 20.0).ceil() as usize;
                packets.push(ServerboundPacket::DigBlock {
                    status: minecraft_format::blocks::DiggingState::Started,
                    location: Position { x, y: y as i16, z },
                    face: minecraft_format::blocks::BlockFace::Top,
                });

                self.state = DigDownState::WaitDigging { ticks };
            }
            DigDownState::WaitDigging { ticks } => {
                if *ticks >= 1 {
                    self.state = DigDownState::WaitDigging { ticks: *ticks - 1 };
                } else {
                    self.state = DigDownState::FinishDigging;
                }
            }
            DigDownState::FinishDigging => {
                let (x, y, z) = (position.x.floor() as i32, position.y.floor() as i32 - 1, position.z.floor() as i32);

                packets.push(ServerboundPacket::DigBlock {
                    status: minecraft_format::blocks::DiggingState::Finished,
                    location: Position { x, y: y as i16, z },
                    face: minecraft_format::blocks::BlockFace::Top,
                });

                if bot.map.get_block(x, y - 1, z) == Block::Air || bot.map.get_block(x, y - 1, z) == Block::CaveAir {
                    packets.push(ServerboundPacket::DigBlock {
                        status: minecraft_format::blocks::DiggingState::Finished,
                        location: Position { x, y: y as i16, z },
                        face: minecraft_format::blocks::BlockFace::Top,
                    });
                }
                // todo check open blocks

                self.state = DigDownState::FindTool {submission: MoveItemToHotbar::new(1, vec![Item::IronPickaxe, Item::StonePickaxe, Item::WoodenPickaxe], Some(0)) };
            }

            DigDownState::Done => {
                return MissionResult::Done;
            }
            DigDownState::Failed => {
                return MissionResult::Failed;
            }
        }

        MissionResult::InProgress
    }
}
