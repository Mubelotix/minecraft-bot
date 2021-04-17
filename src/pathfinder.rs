use crate::map::Map;
use log::*;
use minecraft_format::ids::blocks::Block;
use std::{cell::RefCell, collections::BTreeMap, thread::current};

// todo listen for block change and update path as the map is modified
// todo consider some blocks as liquid and some as transparent
#[derive(Debug)]
pub struct Path {
    path: Vec<(i32, i32, i32)>,
}

type Accesses = Vec<((i32, i32, i32), usize)>;
type AccessibleBlock = BTreeMap<(i32, i32, i32), ((i32, i32, i32), usize)>;

impl Path {
    #[allow(clippy::short_circuit_statement)]
    #[allow(unused_must_use)]
    pub fn find_accessible_neighbors(
        map: &Map,
        (ax, ay, az): (i32, i32, i32),
        distance: usize,
        accessible_blocks: &mut AccessibleBlock,
        accesses: &mut Accesses,
    ) {
        let accessible_blocks = RefCell::new(accessible_blocks);
        let accesses = RefCell::new(accesses);

        let check_direct_neighbor = |x, z| -> bool {
            if let Some((_, previous_distance)) = accessible_blocks.borrow().get(&(x, ay, z)) {
                if *previous_distance <= distance + 1 {
                    return true;
                }
            }
            if map.get_block(x, ay, z) == Block::Air && map.get_block(x, ay + 1, z) == Block::Air && map.get_block(x, ay - 1, z) != Block::Air {
                accessible_blocks.borrow_mut().insert((x, ay, z), ((ax, ay, az), distance + 1));
                accesses.borrow_mut().push(((x, ay, z), distance + 1));
                return true;
            }
            false
        };

        let check_uphill_neighbor = |x, z| -> bool {
            if let Some((_, previous_distance)) = accessible_blocks.borrow().get(&(x, ay + 1, z)) {
                if *previous_distance <= distance + 1 {
                    return true;
                }
            }
            if map.get_block(x, ay + 1, z) == Block::Air
                && map.get_block(x, ay + 2, z) == Block::Air
                && map.get_block(x, ay, z) != Block::Air
                && map.get_block(ax, ay + 2, az) == Block::Air
            {
                accessible_blocks.borrow_mut().insert((x, ay + 1, z), ((ax, ay, az), distance + 1));
                accesses.borrow_mut().push(((x, ay + 1, z), distance + 1));
                return true;
            }
            false
        };

        let check_downhill_neighbors = |x, z| -> bool {
            'height: for offset in 1..=3 {
                if let Some((_, previous_distance)) = accessible_blocks.borrow().get(&(x, ay - offset, z)) {
                    if *previous_distance <= distance + 1 {
                        return true;
                    }
                }
                for y in ay - offset..=ay + 1 {
                    if map.get_block(x, y, z) != Block::Air {
                        continue 'height;
                    }
                }
                if map.get_block(x, ay - 1 - offset, z) != Block::Air {
                    accessible_blocks.borrow_mut().insert((x, ay - offset, z), ((ax, ay, az), distance + 1));
                    accesses.borrow_mut().push(((x, ay - offset, z), distance + 1));
                    return true;
                }
            }
            false
        };

        check_direct_neighbor(ax + 1, az) || check_uphill_neighbor(ax + 1, az) || check_downhill_neighbors(ax + 1, az);
        check_direct_neighbor(ax - 1, az) || check_uphill_neighbor(ax - 1, az) || check_downhill_neighbors(ax - 1, az);
        check_direct_neighbor(ax, az + 1) || check_uphill_neighbor(ax, az + 1) || check_downhill_neighbors(ax, az + 1);
        check_direct_neighbor(ax, az - 1) || check_uphill_neighbor(ax, az - 1) || check_downhill_neighbors(ax, az - 1);
    }

    // todo favoritize good direction
    pub fn find_path(map: &Map, position: (i32, i32, i32), destination: (i32, i32, i32)) -> Option<Path> {
        let mut accessible_blocks = BTreeMap::new();
        let mut accesses = Vec::new();

        accessible_blocks.insert(position, (position, 0));
        accesses.push((position, 0));

        let mut counter = 0;
        while counter < 5000 {
            let (position, distance) = match accesses.pop() {
                Some(access) => access,
                None => break,
            };
            Self::find_accessible_neighbors(map, position, distance, &mut accessible_blocks, &mut accesses);

            if accessible_blocks.contains_key(&destination) {
                trace!("Found destination!");
                break;
            }

            if counter % 20 == 0 {
                accesses.sort_by_key(|((x, y, z), dis)| -(*dis as i32 * 2) - ((x-destination.0).pow(2) as f64 + (y-destination.1).pow(2) as f64 + (z-destination.2).pow(2) as f64).sqrt() as i32);
            }
            counter += 1;
        }

        trace!(
            "There is at least {} accessible blocks (found in {} iterations)",
            accessible_blocks.len(),
            counter
        );

        let mut path = Vec::new();
        let mut current = accessible_blocks.get(&destination)?;
        loop {
            path.push(current.0);
            if current.0 == position {
                break;
            }
            current = accessible_blocks.get(&current.0).unwrap();
        }
        path.reverse();

        Some(Path { path })
    }

    pub fn follow(&self, mut position: (f64, f64, f64), map: &Map) -> Option<((f64, f64), bool)> {
        let x = position.0.floor() as i32;
        let y = position.1.floor() as i32;
        let z = position.2.floor() as i32;
        let mut jump = false;

        let mut current_idx = None;
        for (idx, position) in self.path.iter().enumerate() {
            if position.0 == x && position.2 == z && (y-2..=y).contains(&position.1) {
                current_idx = Some(idx)
            }
        }
        let current_idx = match current_idx {
            Some(current_idx) => current_idx,
            None => {
                warn!("Out of path. Current position {} {} {} not found", x, y, z);
                return None;
            }
        };

        let next_position = match self.path.get(current_idx + 1) {
            Some(next_position) => *next_position,
            None => {
                warn!("Destination achieved! No path fragment after idx {}", current_idx + 1);
                return None;
            }
        };

        if next_position.1 > y && map.is_on_ground(position.0, position.1, position.2) {
            jump = true;
        }
        match (next_position.0 as f64 + 0.5).partial_cmp(&position.0) {
            Some(std::cmp::Ordering::Less) => {
                if map.can_move_west(position.0, position.1, position.2) {
                    position.0 -= 0.2;
                } else {
                    trace!("cannot move west");
                }
            }
            Some(std::cmp::Ordering::Greater) => {
                if map.can_move_east(position.0, position.1, position.2) {
                    position.0 += 0.2;
                } else {
                    trace!("cannot move east");
                }
            }
            _ => {}
        }
        match (next_position.2 as f64 + 0.5).partial_cmp(&position.2) {
            Some(std::cmp::Ordering::Less) => {
                if map.can_move_north(position.0, position.1, position.2) {
                    position.2 -= 0.2;
                } else {
                    trace!("cannot move north");
                }
            }
            Some(std::cmp::Ordering::Greater) => {
                if map.can_move_south(position.0, position.1, position.2) {
                    position.2 += 0.2;
                } else {
                    trace!("cannot move south");
                }
            }
            _ => {}
        }
        

        Some(((position.0, position.2), jump))
    }
}
