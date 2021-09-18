use std::collections::HashMap;

use crate::*;
use minecraft_bot_macros::tick_distributed;

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

#[tick_distributed]
pub fn cut_trees(wood_goal: usize, sappling_goal: usize, mt_bot: &mut Bot, mt_packets: &mut Vec<ServerboundPacket>) -> Result<String, String> {
    // Find trees
    let (px, py, pz): (i32, i32, i32) = (
        bot.position.as_ref().unwrap().x as i32,
        bot.position.as_ref().unwrap().y as i32,
        bot.position.as_ref().unwrap().z as i32,
    );
    let mut wood_blocks: Vec<(i32, i32, i32)> = bot.map.search_blocks(px, pz, &[Block::OakLog, Block::BirchLog], wood_goal * 5, 32 * 32);
    {
        let mut trees: HashMap<(i32, i32), i32> = HashMap::new();
        for wood_block in wood_blocks {
            if let Some(previous_tree) = trees.get(&(wood_block.0, wood_block.2)) {
                if *previous_tree < wood_block.1 {
                    continue;
                }
            }
            trees.insert((wood_block.0, wood_block.2), wood_block.1);
        }
        wood_blocks = trees.into_iter().map(|(k, v)| (k.0, v, k.1)).collect();
        wood_blocks.sort_by_key(|(x, y, z)| -((x - px).abs() + (y - py).abs() + (z - pz).abs()));
    }

    'mt_cut_tree: loop {
        // Count current ressources
        let mut log_count: usize = 0;
        let mut sappling_count: usize = 0;
        for slot in bot.windows.player_inventory.get_slots() {
            if let Some(item) = &slot.item {
                if WOOD_ITEMS.contains(&item.item_id) {
                    log_count += item.item_count.0 as usize;
                } else if SAPLING_ITEMS.contains(&item.item_id) {
                    sappling_count += item.item_count.0 as usize;
                }
            }
        }

        // Check for needs
        if log_count >= wood_goal && sappling_count >= sappling_goal {
            break 'mt_cut_tree;
        }

        // Pick up items
        let mut items: Vec<(i32, i32, i32)> = bot.entities.get_items(Some(&WOOD_ITEMS));
        'mt_pick_item: loop {
            let (ix, iy, iz): (i32, i32, i32) = match items.pop() {
                Some(item) => item,
                None => break 'mt_pick_item,
            };
            
            let result: Result<String, String> = mt_travel((ix, iy, iz), 1000);
        }

        // Select tree
        let (mut dx, mut dy, mut dz, tx, mut ty, tz): (i32, i32, i32, i32, i32, i32) = 'select_tree: loop {
            let (tx, ty, tz) = match wood_blocks.pop() {
                Some(candidate) => candidate,
                None => {
                    return Err("No tree left".to_string());
                }
            };

            if bot.map.get_block(tx, ty - 1, tz).is_blocking() {
                for (dx, dz) in &[(tx - 1, tz), (tx + 1, tz), (tx, tz - 1), (tx, tz + 1)] {
                    let (dx, dz) = (*dx, *dz);

                    if bot.map.get_block(dx, ty - 1, dz).is_blocking()
                        && bot.map.get_block(dx, ty, dz).is_air_block()
                        && bot.map.get_block(dx, ty + 1, dz).is_air_block()
                    {
                        break 'select_tree (dx, ty, dz, tx, ty, tz);
                    }
                }
            }
        };

        'mt_cut_wood_block: loop {
            // Move to tree
            let travel_mission: Result<String, String> = mt_travel((dx, dy, dz), 5000);

            // Start cutting
            packets.push(ServerboundPacket::DigBlock {
                status: minecraft_protocol::components::blocks::DiggingState::Started,
                location: Position { x: tx, y: ty as i16, z: tz },
                face: minecraft_protocol::components::blocks::BlockFace::Top,
            });
            let mut ticks: usize = 3 * 20;

            // Wait for finish
            'mt_wait_cutting: loop {
                if ticks >= 1 {
                    ticks -= 1;
                } else {
                    break 'mt_wait_cutting;
                }
            }

            // Finish cutting
            packets.push(ServerboundPacket::DigBlock {
                status: minecraft_protocol::components::blocks::DiggingState::Finished,
                location: Position { x: tx, y: ty as i16, z: tz },
                face: minecraft_protocol::components::blocks::BlockFace::Top,
            });
            bot.map.set_block(tx, ty, tz, Block::Air);

            // Look for wood blocks above
            if [Block::OakLog, Block::BirchLog].contains(&bot.map.get_block(tx, ty + 1, tz)) {
                if (dx == tx && dz == tz) || (bot.map.get_block(tx, ty - 2, tz).is_blocking() && bot.map.get_block(tx, ty - 1, tz).is_air_block()) {
                    ty += 1;
                    dx = tx;
                    dz = tz;
                    continue 'mt_cut_wood_block;
                } else if tx - dx <= 2 {
                    ty += 1;
                    continue 'mt_cut_wood_block;
                } else {
                    break 'mt_cut_wood_block;
                }
            }

            break 'mt_cut_wood_block;
        }
    }

    Ok("".to_string())
}
