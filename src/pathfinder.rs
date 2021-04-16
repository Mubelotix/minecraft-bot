use crate::map::Map;
use log::*;
use minecraft_format::ids::blocks::Block;
use std::{borrow::BorrowMut, collections::BTreeMap};

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
        let accessible_blocks = std::cell::RefCell::new(accessible_blocks);
        let accesses = std::cell::RefCell::new(accesses);

        let check_direct_neighbor = |x, z| -> bool {
            // TODO Check for better distance
            if accessible_blocks.borrow().contains_key(&(x, ay, z)) {
                return true;
            }
            if map.get_block(x, ay, z) == Block::Air && map.get_block(x, ay + 1, z) == Block::Air && map.get_block(x, ay - 1, z) != Block::Air {
                accessible_blocks.borrow_mut().insert((x, ay, z), ((ax, ay, az), distance + 1));
                accesses.borrow_mut().push(((x, ay, z), distance + 1));
                return true;
            }
            false
        };

        let check_uphill_neighbor = |x, z| -> bool {
            // TODO Check for better distance
            if accessible_blocks.borrow().contains_key(&(x, ay + 1, z)) {
                return true;
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
                // TODO Check for better distance
                if accessible_blocks.borrow().contains_key(&(x, ay - offset, z)) {
                    return true;
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

            if counter % 40 == 0 {
                accesses.sort_by_key(|(_, dis)| -(*dis as i32));
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
            if current.0 == position {
                break;
            }

            path.push(current.0);
            current = accessible_blocks.get(&current.0).unwrap();
        }

        Some(Path { path })
    }
}
