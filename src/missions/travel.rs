use crate::{bot::PlayerPosition, map::Map, pathfinder::Path};

#[derive(Debug)]
pub struct TravelMission {
    path: Path,
}

impl TravelMission {
    pub fn new(destination: (i32, i32, i32), map: &Map, position: (i32, i32, i32)) -> Option<Self> {
        Some(Self {
            path: Path::find_path(map, position, destination)?
        })
    }

    pub fn apply(&mut self, vertical_speed: &mut f64, map: &Map, position: &mut PlayerPosition) {
        if let Some(((x, z), jump)) = self.path.follow((position.x, position.y, position.z), &map) {
            position.x = x;
            position.z = z;
            if jump {
                *vertical_speed = 0.4;
            }
        }
    }
}