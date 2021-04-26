use std::collections::HashMap;

use crate::*;

enum State {
    CheckNeed,
    FindTrees,
    SelectTree,
    MoveToTree { mission: TravelMission, x: i32, y: i32, z: i32 },
    StartDigTree { x: i32, y: i32, z: i32 },
    ContinueDigTree { ticks: u8, x: i32, y: i32, z: i32 },
    FinishDigTree { x: i32, y: i32, z: i32 },
    FindItems,
    SelectItem,
    MoveToItem {mission: TravelMission},

    Failed,
    Done,
}

use State::*;

pub struct SettlementMission {
    state: State,
    trees: Option<Vec<(i32, i32, i32)>>,
    items: Vec<(i32, i32, i32)>
}

const WOOD_ITEMS: [Item; 14] = [
    Item::StrippedOakWood,
    Item::StrippedSpruceWood,
    Item::StrippedBirchWood,
    Item::StrippedJungleWood,
    Item::StrippedAcaciaWood,
    Item::StrippedDarkOakWood,
    Item::StrippedCrimsonHyphae,
    Item::StrippedWarpedHyphae,
    Item::OakWood,
    Item::SpruceWood,
    Item::BirchWood,
    Item::JungleWood,
    Item::AcaciaWood,
    Item::DarkOakWood,
];

const SEED_ITEMS: [Item; 2] = [
    Item::OakSapling,
    Item::BirchSapling,
    // incomplete since we don't want the others
];

impl SettlementMission {
    pub fn new() -> Self {
        SettlementMission {
            state: CheckNeed,
            trees: None,
            items: Vec::new()
        }
    }
}

impl Mission for SettlementMission {
    fn execute(&mut self, bot: &mut Bot, packets: &mut Vec<ServerboundPacket>) -> MissionResult {
        let position = match &bot.position {
            Some(position) => (position.x, position.y, position.z),
            None => return MissionResult::Failed,
        };

        match &mut self.state {
            CheckNeed => {
                let mut wood_count = 0;
                let mut sappling_count = 0;
                for slot in bot.windows.player_inventory.get_slots() {
                    if let Some(item) = &slot.item {
                        if WOOD_ITEMS.contains(&item.item_id) {
                            wood_count += item.item_count.0;
                        }
                        if SEED_ITEMS.contains(&item.item_id) {
                            sappling_count += item.item_count.0;
                        }
                    }
                }

                if sappling_count < 3 || wood_count < 30 {
                    self.state = FindTrees;
                } else {
                    self.state = Done;
                    return MissionResult::Done;
                }
            }
            FindTrees => {
                let wood_blocks = bot
                    .map
                    .search_blocks(position.0 as i32, position.2 as i32, &[Block::OakLog, Block::BirchLog], 500, 32*32);
                let mut trees = HashMap::new();
                for wood_block in wood_blocks {
                    if let Some(previous_tree) = trees.get(&(wood_block.0, wood_block.2)) {
                        if *previous_tree < wood_block.1 {
                            continue;
                        }
                    }
                    trees.insert((wood_block.0, wood_block.2), wood_block.1);
                }
                let trees = trees.into_iter().map(|(k, v)| (k.0, v, k.1)).collect();

                self.trees = Some(trees);
                self.state = SelectTree;
            }
            SelectTree => {
                let trees = match self.trees.as_mut() {
                    Some(trees) => trees,
                    None => {
                        warn!("Selecting tree but there trees have not been searched");
                        self.state = FindTrees;
                        return MissionResult::InProgress;
                    }
                };

                trees.sort_by_key(|(x, y, z)| -((x - position.0 as i32).abs() + (y - position.1 as i32).abs() + (z - position.2 as i32).abs()));

                loop {
                    let (x, y, z) = match trees.pop() {
                        Some(candidate) => candidate,
                        None => {
                            warn!("No tree candidate left!");
                            self.state = Failed;
                            return MissionResult::Failed;
                        }
                    };

                    if bot.map.get_block(x, y - 1, z).is_blocking() {
                        if bot.map.get_block(x - 1, y - 1, z).is_blocking()
                            && bot.map.get_block(x - 1, y, z).is_air_block()
                            && bot.map.get_block(x - 1, y + 1, z).is_air_block()
                        {
                            if let Some(mission) = TravelMission::new(
                                &bot.map,
                                (position.0.floor() as i32, position.1.floor() as i32, position.2.floor() as i32),
                                (x - 1, y, z),
                            ) {
                                self.state = MoveToTree { mission, x, y, z };
                            }
                            return MissionResult::InProgress;
                        }
                        if bot.map.get_block(x + 1, y - 1, z).is_blocking()
                            && bot.map.get_block(x + 1, y, z).is_air_block()
                            && bot.map.get_block(x + 1, y + 1, z).is_air_block()
                        {
                            if let Some(mission) = TravelMission::new(
                                &bot.map,
                                (position.0.floor() as i32, position.1.floor() as i32, position.2.floor() as i32),
                                (x + 1, y, z),
                            ) {
                                self.state = MoveToTree { mission, x, y, z };
                            }
                            return MissionResult::InProgress;
                        }
                        if bot.map.get_block(x, y - 1, z - 1).is_blocking()
                            && bot.map.get_block(x, y, z - 1).is_air_block()
                            && bot.map.get_block(x, y + 1, z - 1).is_air_block()
                        {
                            if let Some(mission) = TravelMission::new(
                                &bot.map,
                                (position.0.floor() as i32, position.1.floor() as i32, position.2.floor() as i32),
                                (x, y, z - 1),
                            ) {
                                self.state = MoveToTree { mission, x, y, z };
                            }
                            return MissionResult::InProgress;
                        }
                        if bot.map.get_block(x, y - 1, z + 1).is_blocking()
                            && bot.map.get_block(x, y, z + 1).is_air_block()
                            && bot.map.get_block(x, y + 1, z + 1).is_air_block()
                        {
                            if let Some(mission) = TravelMission::new(
                                &bot.map,
                                (position.0.floor() as i32, position.1.floor() as i32, position.2.floor() as i32),
                                (x, y, z + 1),
                            ) {
                                self.state = MoveToTree { mission, x, y, z };
                            }
                            return MissionResult::InProgress;
                        }
                    }
                }
            }
            MoveToTree { mission, x, y, z } => match mission.execute(bot, packets) {
                MissionResult::InProgress => (),
                MissionResult::Done => self.state = StartDigTree { x: *x, y: *y, z: *z },
                MissionResult::Failed => self.state = SelectTree,
            },
            StartDigTree { x, y, z } => {
                packets.push(ServerboundPacket::DigBlock {
                    status: minecraft_format::blocks::DiggingState::Started,
                    location: Position { x: *x, y: *y as i16, z: *z },
                    face: minecraft_format::blocks::BlockFace::Top,
                });

                self.state = ContinueDigTree {
                    ticks: 3 * 20,
                    x: *x,
                    y: *y,
                    z: *z,
                };
            }
            ContinueDigTree { ticks, x, y, z } => {
                if *ticks == 0 {
                    self.state = FinishDigTree { x: *x, y: *y, z: *z };
                } else {
                    *ticks -= 1;
                }
            }
            FinishDigTree { x, y, z } => {
                packets.push(ServerboundPacket::DigBlock {
                    status: minecraft_format::blocks::DiggingState::Finished,
                    location: Position { x: *x, y: *y as i16, z: *z },
                    face: minecraft_format::blocks::BlockFace::Top,
                });
                bot.map.set_block(*x, *y, *z, Block::Air);
                bot.windows.player_inventory.use_held_item(1);

                if [Block::OakLog, Block::BirchLog].contains(&bot.map.get_block(*x, *y + 1, *z)) {
                    if (position.0.floor() as i32 != *x || position.2.floor() as i32 != *z)
                        && bot.map.get_block(*x, *y - 1, *z).is_air_block()
                        && bot.map.get_block(*x, *y - 2, *z).is_blocking()
                    {
                        if let Some(mission) = TravelMission::new(
                            &bot.map,
                            (position.0.floor() as i32, position.1.floor() as i32, position.2.floor() as i32),
                            (*x, *y - 1, *z),
                        ) {
                            self.state = MoveToTree {
                                mission,
                                x: *x,
                                y: *y + 1,
                                z: *z,
                            };
                        } else {
                            warn!("Failed to find path to tree but the destination is one block away and there should be no obstacle.");
                            self.state = FindItems;
                        }
                    } else {
                        self.state = StartDigTree { x: *x, y: *y + 1, z: *z };
                    }
                } else {
                    self.state = FindItems;
                }
            }
            FindItems => {
                self.items = bot.entities.get_items(Some(&[Item::OakLog, Item::BirchLog]));
                self.state = SelectItem;
            }
            SelectItem => {
                loop {
                    let item = match self.items.pop() {
                        Some(item) => item,
                        None => {
                            self.state = SelectTree;
                            break;
                        }
                    };
                    if let Some(mission) = TravelMission::new(&bot.map, (position.0.floor() as i32, position.1.floor() as i32, position.2.floor() as i32), item) {
                        self.state = MoveToItem {mission};
                        break;
                    }
                }
            } 
            MoveToItem {mission} => {
                match mission.execute(bot, packets) {
                    MissionResult::InProgress => (),
                    MissionResult::Done => self.state = SelectItem,
                    MissionResult::Failed => self.state = SelectItem,
                }
            }

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
