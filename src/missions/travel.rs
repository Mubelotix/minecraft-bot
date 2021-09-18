use crate::*;
use minecraft_bot_macros::tick_distributed;
use std::cmp::Ordering;

#[tick_distributed]
pub fn travel(destination: (i32, i32, i32), maximum_work_allowed: usize, mt_bot: &mut Bot, mt_packets: &mut Vec<ServerboundPacket>) -> Result<String, String> {
    let mut path_finding_attempt: Option<Vec<(i32, i32, i32)>> = find_path(
        &bot.map,
        (
            bot.position.as_ref().unwrap().x.floor() as i32,
            bot.position.as_ref().unwrap().y.floor() as i32,
            bot.position.as_ref().unwrap().z.floor() as i32,
        ),
        destination,
        maximum_work_allowed,
    );

    let mut path: Vec<(i32, i32, i32)> = match path_finding_attempt.clone() {
        Some(path) => path,
        None => return Err("No path found".to_string()),
    };

    let mut stucked_detector: usize = 0;

    'mt_travel: loop {
        let (mut x, y, mut z): (f64, f64, f64) = (
            bot.position.as_ref().unwrap().x,
            bot.position.as_ref().unwrap().y,
            bot.position.as_ref().unwrap().z,
        );
        let (bx, by, bz): (i32, i32, i32) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
        let on_ground: bool = bot.map.is_on_ground(x, y, z);
        let mut jump: bool = false;

        if stucked_detector > 100 {
            warn!("Bot is stucked while traveling. Recalculating...");
            path_finding_attempt = find_path(
                &bot.map,
                (
                    bot.position.as_ref().unwrap().x.floor() as i32,
                    bot.position.as_ref().unwrap().y.floor() as i32,
                    bot.position.as_ref().unwrap().z.floor() as i32,
                ),
                destination,
                maximum_work_allowed,
            );

            match path_finding_attempt.clone() {
                Some(path) => path,
                None => return Err("No path found".to_string()),
            };

            stucked_detector = 0;
        }

        let (nx, ny, nz): (i32, i32, i32) = match path.get(0) {
            Some(next) => *next,
            None => return Ok("Travel finished".to_string()),
        };

        if nx == bx && ny == by && nz == bz {
            path.remove(0);
            stucked_detector = 0;
            continue 'mt_travel;
        }

        if ny > by && on_ground {
            jump = true;
        }

        let mut movement_required: f64 = (nx as f64 + 0.5 - x).abs();
        if movement_required > 0.2 {
            movement_required = 0.2;
        }
        match (nx as f64 + 0.5).partial_cmp(&x) {
            Some(Ordering::Less) => {
                let max = bot.map.max_west_movement(x, y, z);
                x -= if max > movement_required { movement_required } else { max };
            }
            Some(Ordering::Greater) => {
                let max = bot.map.max_east_movement(x, y, z);
                x += if max > movement_required { movement_required } else { max };
            }
            _ => {}
        }

        let mut movement_required: f64 = (nz as f64 + 0.5 - z).abs();
        if movement_required > 0.2 {
            movement_required = 0.2;
        }
        match (nz as f64 + 0.5).partial_cmp(&z) {
            Some(Ordering::Less) => {
                let max = bot.map.max_north_movement(x, y, z);
                z -= if max > movement_required { movement_required } else { max };
            }
            Some(Ordering::Greater) => {
                let max = bot.map.max_south_movement(x, y, z);
                z += if max > movement_required { movement_required } else { max };
            }
            _ => {}
        }

        let yaw: f32 = match ((nx as f64 + 0.5).partial_cmp(&x), (nz as f64 + 0.5).partial_cmp(&z)) {
            (None, None) | (Some(Ordering::Equal), Some(Ordering::Equal)) | (None, Some(Ordering::Equal)) | (Some(Ordering::Equal), None) => {
                bot.position.as_ref().unwrap().yaw
            }
            (None, Some(Ordering::Less)) | (Some(Ordering::Equal), Some(Ordering::Less)) => 180.0,
            (None, Some(Ordering::Greater)) | (Some(Ordering::Equal), Some(Ordering::Greater)) => 0.0,
            (Some(Ordering::Less), None) | (Some(Ordering::Less), Some(Ordering::Equal)) => 90.0,
            (Some(Ordering::Greater), None) | (Some(Ordering::Greater), Some(Ordering::Equal)) => 270.0,
            (Some(Ordering::Less), Some(Ordering::Less)) => 135.0,
            (Some(Ordering::Less), Some(Ordering::Greater)) => 45.0,
            (Some(Ordering::Greater), Some(Ordering::Less)) => 225.0,
            (Some(Ordering::Greater), Some(Ordering::Greater)) => 315.0,
        };
        packets.push(ServerboundPacket::PlayerRotation { yaw, pitch: 0.0, on_ground });

        stucked_detector += 1;
        bot.position.as_mut().unwrap().x = x;
        bot.position.as_mut().unwrap().z = z;
        if jump {
            bot.vertical_speed = 0.4;
        }
    }

    Ok("Travel finished".to_string())
}
