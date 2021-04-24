use crate::map::Map;
use log::*;
use minecraft_format::packets::Direction;
use std::{cell::RefCell, collections::{BTreeMap, BinaryHeap}};

// todo listen for block change and update path as the map is modified
// todo consider some blocks as liquid and some as transparent
#[derive(Debug)]
pub struct Path {
    path: Vec<(i32, i32, i32)>,
    can_log: bool,
}

type Accesses = Vec<((i32, i32, i32), usize)>;
type AccessibleBlock = BTreeMap<(i32, i32, i32), ((i32, i32, i32), usize)>;

#[derive(Debug, PartialEq, Eq)]
pub struct Node {
    g_cost: usize,
    h_cost: usize,
    x: i32,
    y: i32,
    z: i32,
    origin: (i32, i32, i32),
}

impl Node {
    fn new(position: (i32, i32, i32), origin: (i32, i32, i32), destination: (i32, i32, i32), g_cost: usize) -> Node {
        let delta_x = destination.0 - position.0;
        let delta_y = destination.1 - position.1;
        let delta_z = destination.2 - position.2;
        let h_cost = (((delta_x as f64).powi(2) + (delta_y as f64).powi(2) + (delta_z as f64).powi(2)).sqrt() * 10.0) as usize;
        Node {
            g_cost,
            h_cost,
            x: position.0,
            y: position.1,
            z: position.2,
            origin
        }
    }

    #[inline]
    fn f_cost(&self) -> usize {
        self.g_cost + self.h_cost
    }

    #[inline]
    fn check_direct_neighbor(map: &Map, x: i32, ay: i32, z: i32) -> bool {
        map.get_block(x, ay, z).is_air_block() && map.get_block(x, ay + 1, z).is_air_block() && map.get_block(x, ay - 1, z).is_blocking()
    }

    #[inline]
    fn check_uphill_neighbor(map: &Map, x: i32, ay: i32, z: i32, ax: i32, az: i32) -> bool {
        map.get_block(x, ay + 1, z).is_air_block()
            && map.get_block(x, ay + 2, z).is_air_block()
            && map.get_block(x, ay, z).is_blocking()
            && map.get_block(ax, ay + 2, az).is_air_block()
    }

    #[allow(clippy::short_circuit_statement)]
    #[allow(unused_must_use)]
    pub fn close(
        self,
        map: &Map,
        destination: (i32, i32, i32),
        closed_nodes: &mut Vec<Node>,
        open_nodes: &mut BinaryHeap<Node>,
    ) {
        if closed_nodes.iter().any(|e| e.x == self.x && e.z == self.z && e.y == self.y && e.g_cost <= self.g_cost) {
            return;
        }

        let open_nodes = RefCell::new(open_nodes);

        let add_direct_neighbor = |x, z| -> bool {
            if Node::check_direct_neighbor(map, x, self.y, z) {
                open_nodes.borrow_mut().push(Node::new((x, self.y, z), (self.x, self.y, self.z), destination, self.g_cost + 10));
                return true;
            }
            false
        };

        let add_uphill_neighbor = |x, z| -> bool {
            if Node::check_uphill_neighbor(map, x, self.y, z, self.x, self.z) {
                open_nodes.borrow_mut().push(Node::new((x, self.y + 1, z), (self.x, self.y, self.z), destination, self.g_cost + 10));
                return true;
            }
            false
        };

        let add_downhill_neighbors = |x, z| -> bool {
            'height: for offset in 1..=3 {
                for y in self.y - offset..=self.y + 1 {
                    if map.get_block(x, y, z).is_blocking() {
                        continue 'height;
                    }
                }
                if map.get_block(x, self.y - 1 - offset, z).is_blocking() {
                    open_nodes.borrow_mut().push(Node::new((x, self.y - offset, z), (self.x, self.y, self.z), destination, self.g_cost + 10));
                    return true;
                }
            }
            false
        };

        add_direct_neighbor(self.x + 1, self.z) || add_uphill_neighbor(self.x + 1, self.z) || add_downhill_neighbors(self.x + 1, self.z);
        add_direct_neighbor(self.x - 1, self.z) || add_uphill_neighbor(self.x - 1, self.z) || add_downhill_neighbors(self.x - 1, self.z);
        add_direct_neighbor(self.x, self.z + 1) || add_uphill_neighbor(self.x, self.z + 1) || add_downhill_neighbors(self.x, self.z + 1);
        add_direct_neighbor(self.x, self.z - 1) || add_uphill_neighbor(self.x, self.z - 1) || add_downhill_neighbors(self.x, self.z - 1);

        closed_nodes.push(self);
    }
}

impl std::cmp::PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.f_cost().partial_cmp(&other.f_cost()).map(|o| o.reverse())
    }
}

impl std::cmp::Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.f_cost().cmp(&other.f_cost()).reverse()
    }
}

impl Path {
    pub fn check_direct_neighbor(map: &Map, x: i32, ay: i32, z: i32) -> bool {
        map.get_block(x, ay, z).is_air_block() && map.get_block(x, ay + 1, z).is_air_block() && map.get_block(x, ay - 1, z).is_blocking()
    }

    pub fn check_uphill_neighbor(map: &Map, x: i32, ay: i32, z: i32, ax: i32, az: i32) -> bool {
        map.get_block(x, ay + 1, z).is_air_block()
            && map.get_block(x, ay + 2, z).is_air_block()
            && map.get_block(x, ay, z).is_blocking()
            && map.get_block(ax, ay + 2, az).is_air_block()
    }

    pub fn check_downhill_neighbor(map: &Map, x: i32, ay: i32, z: i32) -> bool {
        map.get_block(x, ay - 1, z).is_air_block()
            && map.get_block(x, ay, z).is_air_block()
            && map.get_block(x, ay + 1, z).is_air_block()
            && map.get_block(x, ay - 2, z).is_blocking()
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
                    if map.get_block(x, y, z).is_blocking() {
                        continue 'height;
                    }
                }
                if map.get_block(x, ay - 1 - offset, z).is_blocking() {
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

    pub fn new_find_path(map: &Map, position: (i32, i32, i32), destination: (i32, i32, i32)) -> Option<Path> {
        let mut closed_nodes: Vec<Node> = Vec::new();
        let mut open_nodes: BinaryHeap<Node> = BinaryHeap::new();
        open_nodes.push(Node::new(position, destination, position, 0));

        let mut counter = 0;
        let start_instant = std::time::Instant::now();
        while let Some(node) = open_nodes.pop() {
            if node.x == destination.0 && node.y == destination.1 && node.z == destination.2 {
                trace!("Found destination");
                break;
            }

            node.close(&map, destination, &mut closed_nodes, &mut open_nodes);

            counter += 1;
            if counter > 7500 {
                trace!("Did not find the destination in time");
                break;
            }
        }
        trace!("Finished the loop in {}ms. {} closed nodes", start_instant.elapsed().as_millis(), closed_nodes.len());

        None
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
