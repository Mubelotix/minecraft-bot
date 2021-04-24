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
        let h_cost = ((delta_x.abs() + delta_y.abs() + delta_z.abs()) as usize) * 12;
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

        let add_direct_neighbor = |x, z, open_nodes: &mut BinaryHeap<Node>| -> bool {
            if Node::check_direct_neighbor(map, x, self.y, z) {
                open_nodes.push(Node::new((x, self.y, z), (self.x, self.y, self.z), destination, self.g_cost + 10));
                return true;
            }
            false
        };

        let add_uphill_neighbor = |x, z, open_nodes: &mut BinaryHeap<Node>| -> bool {
            if Node::check_uphill_neighbor(map, x, self.y, z, self.x, self.z) {
                open_nodes.push(Node::new((x, self.y + 1, z), (self.x, self.y, self.z), destination, self.g_cost + 10));
                return true;
            }
            false
        };

        let add_downhill_neighbors = |x, z, open_nodes: &mut BinaryHeap<Node>| -> bool {
            'height: for offset in 1..=3 {
                for y in self.y - offset..=self.y + 1 {
                    if map.get_block(x, y, z).is_blocking() {
                        continue 'height;
                    }
                }
                if map.get_block(x, self.y - 1 - offset, z).is_blocking() {
                    open_nodes.push(Node::new((x, self.y - offset, z), (self.x, self.y, self.z), destination, self.g_cost + 10));
                    return true;
                }
            }
            false
        };

        let on = open_nodes;
        add_direct_neighbor(self.x + 1, self.z, on) || add_uphill_neighbor(self.x + 1, self.z, on) || add_downhill_neighbors(self.x + 1, self.z, on);
        add_direct_neighbor(self.x - 1, self.z, on) || add_uphill_neighbor(self.x - 1, self.z, on) || add_downhill_neighbors(self.x - 1, self.z, on);
        add_direct_neighbor(self.x, self.z + 1, on) || add_uphill_neighbor(self.x, self.z + 1, on) || add_downhill_neighbors(self.x, self.z + 1, on);
        add_direct_neighbor(self.x, self.z - 1, on) || add_uphill_neighbor(self.x, self.z - 1, on) || add_downhill_neighbors(self.x, self.z - 1, on);

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
    pub fn find_path(map: &Map, position: (i32, i32, i32), destination: (i32, i32, i32)) -> Option<Path> {
        let mut closed_nodes: Vec<Node> = Vec::new();
        let mut open_nodes: BinaryHeap<Node> = BinaryHeap::new();
        open_nodes.push(Node::new(position, destination, position, 0));

        let mut counter = 0;
        let start_instant = std::time::Instant::now();
        while let Some(node) = open_nodes.pop() {
            if node.x == destination.0 && node.y == destination.1 && node.z == destination.2 {
                trace!("Found destination");
                trace!("Finished the loop in {}ms. {} closed nodes", start_instant.elapsed().as_millis(), closed_nodes.len());

                let mut path = Vec::new();
                let mut node = node;
                loop {
                    path.push((node.x, node.y, node.z));
                    let next_node_idx = match closed_nodes.iter().position(|n| n.x == node.origin.0 && n.y == node.origin.1 && n.z == node.origin.2) {
                        Some(idx) => idx,
                        None => break,
                    };
                    node = closed_nodes.remove(next_node_idx);
                }
                path.reverse();
                trace!("Path len: {}", path.len());
        
                return Some(Path { path, can_log: true, })
            }

            node.close(&map, destination, &mut closed_nodes, &mut open_nodes);

            counter += 1;
            if counter > 7500 {
                warn!("Could not find the destination in time. {}ms used", start_instant.elapsed().as_millis());
                return None;
            }
        }

        trace!("Unreachable destination");
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
