use crate::*;
use minecraft_bot_macros::tick_distributed;

#[tick_distributed]
pub fn dig_down(until_block: i32, mt_bot: &mut Bot, mt_packets: &mut Vec<ServerboundPacket>) -> Result<String, String> {
    'mt_placement: loop {
        let mut offset_x: f64 = 0.5 - (bot.position.as_ref().unwrap().x - bot.position.as_ref().unwrap().x.floor());
        let mut offset_z: f64 = 0.5 - (bot.position.as_ref().unwrap().z - bot.position.as_ref().unwrap().z.floor());
        let mut done: bool = true;

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

        bot.position.as_mut().unwrap().x += offset_x;
        bot.position.as_mut().unwrap().z += offset_z;

        if done {
            break 'mt_placement;
        }
    }

    'mt_digging: loop {
        if bot.position.as_ref().unwrap().y.floor() as i32 <= until_block {
            return Ok(format!("Mission complete: Made a hole deeper than {}", until_block))
        }

        let (x, y, z): (i32, i32, i32) = (
            bot.position.as_ref().unwrap().x.floor() as i32,
            bot.position.as_ref().unwrap().y.floor() as i32 - 1,
            bot.position.as_ref().unwrap().z.floor() as i32,
        );

        let block: Block = bot.map.get_block(x, y, z);
        if !block.is_diggable() {
            return Err(format!("Failed to dig, block {:?} is not diggable", block));
        }

        let compatible_harvest_tools: &'static [u32] = block.get_compatible_harvest_tools();
        let (can_harvest, speed_multiplier): (bool, i32) = match &bot.windows.player_inventory.get_hotbar()[0].item {
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

        let mut time_required: f64 = block.get_hardness() as f64;
        match can_harvest {
            true => {
                time_required *= 1.5;
                time_required /= speed_multiplier as f64;
            }
            false => time_required *= 5.0,
        }

        let mut ticks: usize = (time_required * 20.0).ceil() as usize;
        packets.push(ServerboundPacket::DigBlock {
            status: minecraft_protocol::components::blocks::DiggingState::Started,
            location: Position { x, y: y as i16, z },
            face: minecraft_protocol::components::blocks::BlockFace::Top,
        });

        'mt_wait_digging: loop {
            if ticks >= 1 {
                ticks -= 1;
            } else {
                break 'mt_wait_digging;
            }
        }

        packets.push(ServerboundPacket::DigBlock {
            status: minecraft_protocol::components::blocks::DiggingState::Finished,
            location: Position { x, y: y as i16, z },
            face: minecraft_protocol::components::blocks::BlockFace::Top,
        });
        bot.windows.player_inventory.use_held_item(1);

        // TODO Replace blocks
    }

    Err("Loop exited without reason".to_string())
}
