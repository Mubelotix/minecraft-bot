use crate::map::Map;
use log::*;
use minecraft_format::{ids::blocks::Block, packets::Direction};
use std::{cell::RefCell, collections::BTreeMap, thread::current};

// todo listen for block change and update path as the map is modified
// todo consider some blocks as liquid and some as transparent
#[derive(Debug)]
pub struct Path {
    path: Vec<(i32, i32, i32)>,
    can_log: bool,
}

type Accesses = Vec<((i32, i32, i32), usize)>;
type AccessibleBlock = BTreeMap<(i32, i32, i32), ((i32, i32, i32), usize)>;

impl Path {
    pub fn check_direct_neighbor(map: &Map, x: i32, ay: i32, z: i32) -> bool {
        map.get_block(x, ay, z) == Block::Air && map.get_block(x, ay + 1, z) == Block::Air && map.get_block(x, ay - 1, z) != Block::Air
    }

    pub fn check_uphill_neighbor(map: &Map, x: i32, ay: i32, z: i32, ax: i32, az: i32) -> bool {
        map.get_block(x, ay + 1, z) == Block::Air
            && map.get_block(x, ay + 2, z) == Block::Air
            && map.get_block(x, ay, z) != Block::Air
            && map.get_block(ax, ay + 2, az) == Block::Air
    }

    pub fn check_downhill_neighbor(map: &Map, x: i32, ay: i32, z: i32) -> bool {
        map.get_block(x, ay - 1, z) == Block::Air
            && map.get_block(x, ay, z) == Block::Air
            && map.get_block(x, ay + 1, z) == Block::Air
            && map.get_block(x, ay - 2, z) != Block::Air
    }

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

        let add_direct_neighbor = |x, z| -> bool {
            if let Some((_, previous_distance)) = accessible_blocks.borrow().get(&(x, ay, z)) {
                if *previous_distance <= distance + 1 {
                    return true;
                }
            }
            if Path::check_direct_neighbor(map, x, ay, z) {
                accessible_blocks.borrow_mut().insert((x, ay, z), ((ax, ay, az), distance + 1));
                accesses.borrow_mut().push(((x, ay, z), distance + 1));
                return true;
            }
            false
        };

        let add_uphill_neighbor = |x, z| -> bool {
            if let Some((_, previous_distance)) = accessible_blocks.borrow().get(&(x, ay + 1, z)) {
                if *previous_distance <= distance + 1 {
                    return true;
                }
            }
            if Self::check_uphill_neighbor(map, x, ay, z, ax, az) {
                accessible_blocks.borrow_mut().insert((x, ay + 1, z), ((ax, ay, az), distance + 1));
                accesses.borrow_mut().push(((x, ay + 1, z), distance + 1));
                return true;
            }
            false
        };

        let add_downhill_neighbors = |x, z| -> bool {
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

        add_direct_neighbor(ax + 1, az) || add_uphill_neighbor(ax + 1, az) || add_downhill_neighbors(ax + 1, az);
        add_direct_neighbor(ax - 1, az) || add_uphill_neighbor(ax - 1, az) || add_downhill_neighbors(ax - 1, az);
        add_direct_neighbor(ax, az + 1) || add_uphill_neighbor(ax, az + 1) || add_downhill_neighbors(ax, az + 1);
        add_direct_neighbor(ax, az - 1) || add_uphill_neighbor(ax, az - 1) || add_downhill_neighbors(ax, az - 1);
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
                accesses.sort_by_key(|((x, y, z), dis)| {
                    -(*dis as i32 * 2)
                        - ((x - destination.0).pow(2) as f64 + (y - destination.1).pow(2) as f64 + (z - destination.2).pow(2) as f64).sqrt() as i32
                });
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

        Some(Path { path, can_log: false, })
    }

    pub fn new_naive(map: &Map, destination: (i32, i32, i32), (mut ax, mut ay, mut az): (i32, i32, i32)) -> Self {
        let mut working_path = Vec::new();
        let mut current_direction = Direction::North;
        let mut unwanted_turn_right = 0;
        working_path.push(((ax, ay, az), unwanted_turn_right));
        

        for _i in 0..600 {
            if unwanted_turn_right >= 1 {
                let potential_direction = match current_direction {
                    Direction::South => Direction::East,
                    Direction::West => Direction::South,
                    Direction::North => Direction::West,
                    Direction::East => Direction::North,
                };

                let (x, z) = match potential_direction {
                    Direction::South => (ax, az+1),
                    Direction::West => (ax - 1, az),
                    Direction::North => (ax, az - 1),
                    Direction::East => (ax + 1, az),
                };

                if Path::check_direct_neighbor(map, x, ay, z) {
                    ax = x;
                    az = z;
                    current_direction = potential_direction;
                    unwanted_turn_right -= 1;
                    working_path.push(((ax, ay, az), unwanted_turn_right));
                } else if Path::check_uphill_neighbor(map, x, ay, z, ax, az) {
                    ax = x;
                    ay += 1;
                    az = z;
                    current_direction = potential_direction;
                    unwanted_turn_right -= 1;
                    working_path.push(((ax, ay, az), unwanted_turn_right));
                } else if Path::check_downhill_neighbor(map, x, ay, z) {
                    ax = x;
                    ay -= 1;
                    az = z;
                    current_direction = potential_direction;
                    unwanted_turn_right -= 1;
                    working_path.push(((ax, ay, az), unwanted_turn_right));
                }
            }

            let (x, z) = match current_direction {
                Direction::South => (ax, az+1),
                Direction::West => (ax - 1, az),
                Direction::North => (ax, az - 1),
                Direction::East => (ax + 1, az),
            };

            if Path::check_direct_neighbor(map, x, ay, z) {
                ax = x;
                az = z;
                working_path.push(((ax, ay, az), unwanted_turn_right));
            } else if Path::check_uphill_neighbor(map, x, ay, z, ax, az) {
                ax = x;
                ay += 1;
                az = z;
                working_path.push(((ax, ay, az), unwanted_turn_right));
            } else if Path::check_downhill_neighbor(map, x, ay, z) {
                ax = x;
                ay -= 1;
                az = z;
                working_path.push(((ax, ay, az), unwanted_turn_right));
            } else {
                unwanted_turn_right += 1;
                if unwanted_turn_right == 4 {
                    unwanted_turn_right = 0;
                }
                current_direction  = match current_direction {
                    Direction::South => Direction::West,
                    Direction::West => Direction::North,
                    Direction::North => Direction::East,
                    Direction::East => Direction::South,
                }
            }
        }

        trace!("LEN = {}", working_path.len());

        let mut path = Vec::new();
        for (block, direction) in working_path {
            path.push(block)
        }

        Path {path, can_log: false,}
    }

    pub fn follow(&mut self, mut position: (f64, f64, f64), map: &Map) -> Option<((f64, f64), bool)> {
        let x = position.0.floor() as i32;
        let y = position.1.floor() as i32;
        let z = position.2.floor() as i32;
        let mut jump = false;

        let mut current_idx = None;
        for (idx, position) in self.path.iter().enumerate() {
            if position.0 == x && position.2 == z && (y-2..=y).contains(&position.1) {
                current_idx = Some(idx);
                break
            }
        }
        let current_idx = match current_idx {
            Some(current_idx) => current_idx,
            None => {
                if !self.can_log {
                    warn!("Out of path. Current position {} {} {} not found", x, y, z);
                    self.can_log = true;
                }
                return None;
            }
        };

        if self.path.len() > 1 && current_idx > 0 {
            for _ in 0..current_idx {
                self.path.remove(0);
            }
        }

        let next_position = match self.path.get(1) {
            Some(next_position) => {
                *next_position
            },
            None => {
                if !self.can_log {
                    warn!("Destination achieved! No path fragment after idx {}", current_idx + 1);
                    self.can_log = true;
                }
                return None;
            }
        };
        self.can_log = false;

        if next_position.1 > y && map.is_on_ground(position.0, position.1, position.2) {
            jump = true;
        }
        let mut movement_required = (next_position.0 as f64 + 0.5 - position.0).abs();
        if movement_required > 0.2 {
            movement_required = 0.2;
        }
        //trace!("{} x is {} but targets {}", current_idx, position.0, next_position.0 as f64 + 0.5);
        match (next_position.0 as f64 + 0.5).partial_cmp(&position.0) {
            Some(std::cmp::Ordering::Less) => {
                let max = map.max_west_movement(position.0, position.1, position.2);
                position.0 -= if max > movement_required { movement_required } else {max};
            }
            Some(std::cmp::Ordering::Greater) => {
                let max = map.max_east_movement(position.0, position.1, position.2);
                position.0 += if max > movement_required { movement_required } else {max};
            }
            _ => {}
        }

        let mut movement_required = (next_position.2 as f64 + 0.5 - position.2).abs();
        if movement_required > 0.2 {
            movement_required = 0.2;
        }
        match (next_position.2 as f64 + 0.5).partial_cmp(&position.2) {
            Some(std::cmp::Ordering::Less) => {
                let max = map.max_north_movement(position.0, position.1, position.2);
                position.2 -= if max > movement_required { movement_required } else {max};
            }
            Some(std::cmp::Ordering::Greater) => {
                let max = map.max_south_movement(position.0, position.1, position.2);
                position.2 += if max > movement_required { movement_required } else {max};
            }
            _ => {}
        }
        //trace!("z is {} but targets {}", position.2, next_position.2 as f64 + 0.5);
        

        Some(((position.0, position.2), jump))
    }
}
