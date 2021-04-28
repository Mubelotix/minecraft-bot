use std::collections::HashMap;

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectKind {
    Tree,
    Item,
    Wheat,
}

enum State {
    CheckNeed,

    Find(ObjectKind),
    Select(ObjectKind),
    MoveTo {
        object: ObjectKind,
        mission: TravelMission,
        x: i32,
        y: i32,
        z: i32,
    },

    StartDigging {
        object: ObjectKind,
        x: i32,
        y: i32,
        z: i32,
    },
    ContinueDigging {
        object: ObjectKind,
        ticks: u8,
        x: i32,
        y: i32,
        z: i32,
    },
    FinishDigging {
        object: ObjectKind,
        x: i32,
        y: i32,
        z: i32,
    },

    Failed,
    Done,
}

use State::*;

pub struct SettlementMission {
    state: State,
    blocks: Vec<(i32, i32, i32)>,
    items: Vec<(i32, i32, i32)>,
}

const WOOD_ITEMS: [Item; 14] = [
    Item::OakLog,
    Item::SpruceLog,
    Item::BirchLog,
    Item::JungleLog,
    Item::AcaciaLog,
    Item::DarkOakLog,
    Item::CrimsonStem,
    Item::WarpedStem,
    Item::StrippedOakLog,
    Item::StrippedSpruceLog,
    Item::StrippedBirchLog,
    Item::StrippedJungleLog,
    Item::StrippedAcaciaLog,
    Item::StrippedDarkOakLog,
];

const SAPLING_ITEMS: [Item; 2] = [
    Item::OakSapling,
    Item::BirchSapling,
    // incomplete since we don't want the others
];

impl SettlementMission {
    pub fn new() -> Self {
        SettlementMission {
            state: CheckNeed,
            blocks: Vec::new(),
            items: Vec::new(),
        }
    }
}

impl Mission for SettlementMission {
    fn execute(&mut self, bot: &mut Bot, packets: &mut Vec<ServerboundPacket>) -> MissionResult {
        let position = match &bot.position {
            Some(position) => (position.x, position.y, position.z),
            None => return MissionResult::Failed,
        };
        let pos = (position.0.floor() as i32, position.1.floor() as i32, position.2.floor() as i32);

        match &mut self.state {
            CheckNeed => {
                let mut log_count = 0;
                let mut sappling_count = 0;
                let mut wheat_seeds_count = 0;
                for slot in bot.windows.player_inventory.get_slots() {
                    if let Some(item) = &slot.item {
                        if WOOD_ITEMS.contains(&item.item_id) {
                            log_count += item.item_count.0;
                        } else if SAPLING_ITEMS.contains(&item.item_id) {
                            sappling_count += item.item_count.0;
                        } else if item.item_id == Item::WheatSeeds {
                            wheat_seeds_count += item.item_count.0;
                        }
                    }
                }

                if sappling_count < 3 || log_count < 30 {
                    self.state = Find(ObjectKind::Tree);
                } else if wheat_seeds_count < 16 {
                    self.state = Find(ObjectKind::Wheat);
                } else {
                    self.state = Done;
                }
            }
            Find(object) => {
                match object {
                    ObjectKind::Tree => {
                        let wood_blocks = bot.map.search_blocks(pos.0, pos.2, &[Block::OakLog, Block::BirchLog], 750, 32 * 32);
                        let mut trees = HashMap::new();
                        for wood_block in wood_blocks {
                            if let Some(previous_tree) = trees.get(&(wood_block.0, wood_block.2)) {
                                if *previous_tree < wood_block.1 {
                                    continue;
                                }
                            }
                            trees.insert((wood_block.0, wood_block.2), wood_block.1);
                        }
                        self.blocks = trees.into_iter().map(|(k, v)| (k.0, v, k.1)).collect();
                    }
                    ObjectKind::Wheat => {
                        self.blocks = bot.map.search_blocks(pos.0, pos.2, &[Block::Grass, Block::Wheat], 400, 32 * 32);
                    }
                    ObjectKind::Item => {
                        self.items = bot
                            .entities
                            .get_items(Some(&[Item::OakLog, Item::BirchLog, Item::OakSapling, Item::BirchSapling, Item::WheatSeeds]));
                    }
                }

                self.state = Select(*object);
            }
            Select(object) => {
                let list = match object {
                    ObjectKind::Tree => &mut self.blocks,
                    ObjectKind::Wheat => &mut self.blocks,
                    ObjectKind::Item => &mut self.items,
                };
                list.sort_by_key(|(x, y, z)| -((x - pos.0).abs() + (y - pos.1).abs() + (z - pos.2).abs()));

                loop {
                    let (x, y, z) = match list.pop() {
                        Some(candidate) => candidate,
                        None => {
                            trace!("No {:?} candidate left", object);
                            self.state = CheckNeed;
                            return MissionResult::InProgress;
                        }
                    };

                    if bot.map.get_block(x, y - 1, z).is_blocking() {
                        if *object == ObjectKind::Tree {
                            for (nx, nz) in &[(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)] {
                                let (nx, nz) = (*nx, *nz);

                                if bot.map.get_block(nx, y - 1, nz).is_blocking()
                                    && bot.map.get_block(nx, y, nz).is_air_block()
                                    && bot.map.get_block(nx, y + 1, nz).is_air_block()
                                {
                                    if let Some(mission) = TravelMission::new(&bot.map, pos, (nx, y, nz), 5000) {
                                        self.state = MoveTo {
                                            object: *object,
                                            mission,
                                            x,
                                            y,
                                            z,
                                        };
                                        return MissionResult::InProgress;
                                    }
                                }
                            }
                        } else if bot.map.get_block(x, y, z).is_air_block() && bot.map.get_block(x, y + 1, z).is_air_block() {
                            if let Some(mission) = TravelMission::new(&bot.map, pos, (x, y, z), 5000) {
                                self.state = MoveTo {
                                    object: *object,
                                    mission,
                                    x,
                                    y,
                                    z,
                                };
                                return MissionResult::InProgress;
                            }
                        }
                    }
                }
            }
            MoveTo { object, mission, x, y, z } => match mission.execute(bot, packets) {
                MissionResult::InProgress => (),
                MissionResult::Done => match object {
                    ObjectKind::Item => self.state = Select(*object),
                    object => {
                        self.state = StartDigging {
                            object: *object,
                            x: *x,
                            y: *y,
                            z: *z,
                        }
                    }
                },
                MissionResult::Failed => self.state = Select(*object),
            },
            StartDigging { object, x, y, z } => {
                packets.push(ServerboundPacket::DigBlock {
                    status: minecraft_protocol::components::blocks::DiggingState::Started,
                    location: Position { x: *x, y: *y as i16, z: *z },
                    face: minecraft_protocol::components::blocks::BlockFace::Top,
                });

                self.state = ContinueDigging {
                    object: *object,
                    ticks: match object {
                        ObjectKind::Tree => 3 * 20,
                        ObjectKind::Item => {
                            warn!("Digging an item");
                            0
                        }
                        ObjectKind::Wheat => 0,
                    },
                    x: *x,
                    y: *y,
                    z: *z,
                };
            }
            ContinueDigging { object, ticks, x, y, z } => {
                if *ticks == 0 {
                    self.state = FinishDigging {
                        object: *object,
                        x: *x,
                        y: *y,
                        z: *z,
                    };
                } else {
                    *ticks -= 1;
                }
            }
            FinishDigging { object, x, y, z } => {
                packets.push(ServerboundPacket::DigBlock {
                    status: minecraft_protocol::components::blocks::DiggingState::Finished,
                    location: Position { x: *x, y: *y as i16, z: *z },
                    face: minecraft_protocol::components::blocks::BlockFace::Top,
                });
                bot.map.set_block(*x, *y, *z, Block::Air);
                bot.windows.player_inventory.use_held_item(1);

                if *object == ObjectKind::Tree && [Block::OakLog, Block::BirchLog].contains(&bot.map.get_block(*x, *y + 1, *z)) {
                    if (pos.0 != *x || pos.2 != *z) && bot.map.get_block(*x, *y - 1, *z).is_air_block() && bot.map.get_block(*x, *y - 2, *z).is_blocking() {
                        if let Some(mission) = TravelMission::new(&bot.map, pos, (*x, *y - 1, *z), 25) {
                            self.state = MoveTo {
                                object: ObjectKind::Tree,
                                mission,
                                x: *x,
                                y: *y + 1,
                                z: *z,
                            };
                        } else {
                            warn!("Failed to find path to tree but the destination is one block away and there should be no obstacle.");
                            self.state = Find(ObjectKind::Item);
                        }
                    } else {
                        self.state = StartDigging {
                            object: ObjectKind::Tree,
                            x: *x,
                            y: *y + 1,
                            z: *z,
                        };
                    }
                } else {
                    self.state = Find(ObjectKind::Item);
                }
            }
            /*
            // Items
            FindItems => {
                self.items = bot.entities.get_items(Some(&[Item::OakLog, Item::BirchLog]));
                self.state = SelectItem;
            }
            SelectItem => loop {
                let item = match self.items.pop() {
                    Some(item) => item,
                    None => {
                        self.state = CheckNeed;
                        break;
                    }
                };
                if let Some(mission) = TravelMission::new(&bot.map, pos, item, 3000) {
                    self.state = MoveToItem { mission };
                    break;
                }
            },
            MoveToItem { mission } => match mission.execute(bot, packets) {
                MissionResult::InProgress => (),
                MissionResult::Done => self.state = SelectItem,
                MissionResult::Failed => self.state = SelectItem,
            },*/
            Done => {
                return MissionResult::Done;
            }
            Failed => {
                return MissionResult::Failed;
            }
        }

        MissionResult::InProgress
    }
}
