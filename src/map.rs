use log::*;
use minecraft_format::{
    chunk::{ChunkData, ChunkSection},
    ids::blocks::Block,
};
use std::collections::BTreeMap;

pub struct Map {
    chunk_columns: BTreeMap<(i32, i32), [Option<ChunkSection>; 16]>,
}

impl Map {
    pub fn new() -> Map {
        Map {
            chunk_columns: BTreeMap::new(),
        }
    }

    pub fn load_chunk(&mut self, mut chunk_data: ChunkData) {
        let chunk_sections = match chunk_data.deserialize_chunk_sections() {
            Ok(chunk_sections) => chunk_sections,
            Err(e) => {
                error!(
                    "Failed to parse chunk sections at {} {}: {}.",
                    chunk_data.chunk_x, chunk_data.chunk_z, e
                );
                return;
            }
        };
        self.chunk_columns
            .insert((chunk_data.chunk_x, chunk_data.chunk_z), chunk_sections);
        trace!("Loaded chunk {} {}", chunk_data.chunk_x, chunk_data.chunk_z);
    }

    pub fn unload_chunk(&mut self, chunk_x: i32, chunk_z: i32) {
        self.chunk_columns.remove(&(chunk_x, chunk_z));
        trace!("Unloaded chunk {} {}", chunk_x, chunk_z);
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        let x_within_chunk = x.rem_euclid(16);
        let z_within_chunk = z.rem_euclid(16);
        let chunk_x = (x - x_within_chunk) / 16;
        let chunk_z = (z - z_within_chunk) / 16;
        let chunk_column = match self.chunk_columns.get(&(chunk_x, chunk_z)) {
            Some(chunk_column) => chunk_column,
            None => {
                warn!("The indexed block is not loaded");
                return Block::Air;
            }
        };

        if y < 0 {
            warn!("Map indexed with negative y value ({})", y);
            return Block::Air;
        }
        let y_within_chunk = y % 16;
        let chunk_y = (y - y_within_chunk) / 16;
        let chunk_section = match chunk_column.get(chunk_y as usize) {
            Some(Some(chunk_section)) => &chunk_section.blocks,
            Some(None) => return Block::Air,
            None => {
                warn!("Map indexed with out of bound y value");
                return Block::Air;
            }
        };

        let block_state_id = match chunk_section
            .get((y_within_chunk * 16 * 16 + z_within_chunk * 16 + x_within_chunk) as usize)
        {
            Some(block_state) => block_state,
            None => {
                warn!("Missing block in the block array");
                return Block::Air
            },
        };

        match Block::from_state_id(*block_state_id) {
            Some(block) => block,
            None => {
                warn!("Unknown state_id {}", block_state_id);
                Block::Air
            }
        }
    }

    pub fn is_on_ground(&self, x: f64, y: f64, z: f64) -> bool {
        let x_floor = x.floor();
        let x1 = x_floor as i32;
        let x2 = if x - x_floor > 0.7 {
            Some(x1 + 1)
        } else if x - x_floor < 0.3 {
            Some(x1 - 1)
        } else {
            None
        };

        let z_floor = z.floor();
        let z1 = z_floor as i32;
        let z2 = if z - z_floor > 0.7 {
            Some(z1 + 1)
        } else if z - z_floor < 0.3 {
            Some(z1 - 1)
        } else {
            None
        };

        let y = (y - 0.01).floor() as i32;
        if self.get_block(x1, y, z1) != Block::Air {
            return true;
        }
        if let Some(x2) = x2 {
            if self.get_block(x2, y, z1) != Block::Air {
                return true;
            }
            if let Some(z2) = z2 {
                if self.get_block(x2, y, z2) != Block::Air {
                    return true;
                }
            }
        }
        if let Some(z2) = z2 {
            if self.get_block(x1, y, z2) != Block::Air {
                return true;
            }
        }

        false
    }

    pub fn max_fall(&self, x: f64, y: f64, z: f64) -> f64 {
        if self.is_on_ground(x, y, z) {
            return 0.0;
        }
        if self.is_on_ground(x, y - 1.0, z) {
            return (y - 0.01).floor() - (y - 0.01);
        }
        if self.is_on_ground(x, y - 2.0, z) {
            return (y - 0.01).floor() - (y - 0.01) - 1.0;
        }

        -2.0
    }
    
    pub fn set_block_state(
        &mut self,
        chunk_x: i32,
        chunk_y: i32,
        chunk_z: i32,
        block_x: u8,
        block_y: u8,
        block_z: u8,
        block_state_id: u32,
    ) {
        let chunk_column = match self.chunk_columns.get_mut(&(chunk_x, chunk_z)) {
            Some(chunk_column) => chunk_column,
            None => {
                warn!("Block set in a chunk that was not loaded (at {:?})", (chunk_x, chunk_y));
                return;
            }
        };

        let chunk_section = match chunk_column.get_mut(chunk_y as usize) {
            Some(chunk_section) => chunk_section,
            None => {
                warn!("Block set in a chunk that does not exist");
                return;
            }
        };

        let blocks = match chunk_section {
            Some(chunk_section) => &mut chunk_section.blocks,
            None => {
                trace!("Block set in inexistant chunk section: creating a new chunk section");
                *chunk_section = Some(ChunkSection {
                    block_count: 0,
                    palette: None,
                    blocks: vec![0; 16*16*16],
                });
                &mut chunk_section.as_mut().unwrap().blocks
            }
        };

        let idx = block_y as usize * 16 * 16 + block_z as usize * 16 + block_x as usize;
        match blocks.get_mut(idx) {
            Some(old_block) => {
                *old_block = block_state_id;
                // assert_eq!(Block::from_state_id(block_state_id).unwrap(), self.get_block(chunk_x as i32 * 16 + block_x as i32, chunk_y as i32 * 16 + block_y as i32, chunk_z as i32 * 16 + block_z as i32))
            }
            None => {
                warn!("Block does not exist in this chunk section");
            }
        }
    }

    pub fn set_block(
        &mut self,
        chunk_x: i32,
        chunk_y: i32,
        chunk_z: i32,
        block_x: u8,
        block_y: u8,
        block_z: u8,
        block: Block,
    ) {
        let block_state_id = block.get_default_state_id();
        self.set_block_state(chunk_x, chunk_y, chunk_z, block_x, block_y, block_z, block_state_id)
    }
}
