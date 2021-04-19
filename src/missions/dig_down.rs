use minecraft_format::{packets::{play_serverbound::ServerboundPacket, Position}, ids::{blocks::Block, items::Item}};
use log::*;
use crate::{bot::PlayerPosition, map::Map, inventory::Windows};


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

impl DigDownMission {
    pub fn apply(&mut self, position: &mut PlayerPosition, map: &Map, windows: &mut Windows, packets: &mut Vec<ServerboundPacket>) -> bool {
        match self.state {
            DigDownState::MoveToBlockCenter => {
                trace!("Moving to block center");
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
                trace!("{} {} {} {}", position.x, offset_x, position.z, offset_z);
                position.x += offset_x;
                position.z += offset_z;
                if done {
                    self.state = DigDownState::StartDigging;
                }
            }
            DigDownState::FindAppropriateTool => todo!(),
            DigDownState::StartDigging => {
                trace!("Start digging");
                if position.y.floor() as isize <= self.until_block as isize {
                    return true;
                }
                let (x, y, z) = (position.x.floor() as i32, position.y.floor() as i32 - 1, position.z.floor() as i32);
                let block = map.get_block(x, y, z);
                if !block.is_diggable() {
                    error!("Failed to dig, block {:?} is not diggable", block);
                    return true;
                }
                let compatible_harvest_tools = block.get_compatible_harvest_tools();
                let (can_harvest, speed_multiplier) = match windows.player_inventory.get_hotbar()[0] {
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
                trace!("hardness = {}", block.get_hardness() as f64);
                let ticks = (time_required * 20.0).ceil() as usize;
                packets.push(ServerboundPacket::DigBlock {
                    status: minecraft_format::blocks::DiggingState::Started,
                    location: Position { x, y: y as i16, z },
                    face: minecraft_format::blocks::BlockFace::Top,
                });

                trace!("Waiting {} ticks", ticks);
                self.state = DigDownState::WaitDigging { ticks };
            }
            DigDownState::WaitDigging { ticks } => {
                if ticks >= 1 {
                    self.state = DigDownState::WaitDigging { ticks: ticks - 1 };
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

                if map.get_block(x, y - 1, z) == Block::Air || map.get_block(x, y - 1, z) == Block::CaveAir {
                    packets.push(ServerboundPacket::DigBlock {
                        status: minecraft_format::blocks::DiggingState::Finished,
                        location: Position { x, y: y as i16, z },
                        face: minecraft_format::blocks::BlockFace::Top,
                    });
                }
                // todo check open blocks

                self.state = DigDownState::StartDigging;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Copy)]
enum DigDownState {
    MoveToBlockCenter,
    FindAppropriateTool,
    StartDigging,
    WaitDigging { ticks: usize },
    FinishDigging,
}
