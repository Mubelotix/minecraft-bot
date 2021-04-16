use crate::{map::Map, network::connect, pathfinder::Path};
use log::*;
use minecraft_format::{
    blocks::MultiBlockChange,
    chat::ChatMode,
    ids::blocks::Block,
    packets::{play_clientbound::ClientboundPacket, play_serverbound::ServerboundPacket, Position},
    slots::MainHand,
    MinecraftPacketPart,
};
use std::time::Instant;
use std::{
    ops::Mul,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
struct PlayerPosition {
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
}

pub struct Bot {
    username: String,
    addr: String,
    port: u16,
    map: Map,
    self_entity_id: Option<i32>,
    position: Option<PlayerPosition>,
    spawn_position: Option<Position>,
    world_name: Option<String>,

    health: f32,
    food: u32,
    food_saturation: f32,
    vertical_speed: f64,
    path: Option<Path>,
}

impl Bot {
    pub fn create(addr: String, port: u16, username: String) {
        debug!("Connecting {} to {}:{}", username, addr, port);
        let (receiver, sender) = connect(&addr, port, &username);
        info!("{} is connected on {}:{}", username, addr, port);
        let bot = Arc::new(Mutex::new(Bot {
            username,
            addr,
            port,
            map: Map::new(),
            position: None,
            spawn_position: None,
            self_entity_id: None,
            world_name: None,

            health: 11.0,
            food: 11,
            food_saturation: 0.0,
            vertical_speed: 0.0,
            path: None,
        }));
        let bot2 = Arc::clone(&bot);
        let sender2 = sender.clone();

        // Wait for the server to be ready.
        std::thread::sleep(std::time::Duration::from_millis(500));

        sender
            .send(
                ServerboundPacket::ClientSettings {
                    locale: "en_US",
                    render_distance: 32,
                    chat_mode: ChatMode::Enabled,
                    chat_colors_enabled: true,
                    displayed_skin_parts: 127,
                    main_hand: MainHand::Right,
                }
                .serialize_minecraft_packet()
                .unwrap(),
            )
            .unwrap();

        std::thread::spawn(move || loop {
            let mut packet_bytes = receiver.recv().unwrap();
            let packet = match ClientboundPacket::deserialize_uncompressed_minecraft_packet(packet_bytes.as_mut_slice()) {
                Ok(packet) => packet,
                Err(e) => {
                    log::error!("Failed to parse clientbound packet: {:?}. In {:?}", e, packet_bytes);
                    continue;
                }
            };
            let mut bot = bot.lock().unwrap();
            let response_packets = bot.update(packet);
            for response_packet in response_packets {
                let response_packet = match response_packet.serialize_minecraft_packet() {
                    Ok(response_packet) => response_packet,
                    Err(e) => {
                        log::error!("Failed to serialize packet from client {}", e);
                        continue;
                    }
                };
                sender.send(response_packet).unwrap();
            }
        });

        loop {
            let start_time = Instant::now();
            {
                let mut bot = bot2.lock().unwrap();
                let response_packets = bot.act();
                for response_packet in response_packets {
                    let response_packet = match response_packet.serialize_minecraft_packet() {
                        Ok(response_packet) => response_packet,
                        Err(e) => {
                            log::error!("Failed to serialize packet from client {}", e);
                            continue;
                        }
                    };
                    sender2.send(response_packet).unwrap();
                }
            }
            let elapsed_time = Instant::now() - start_time;
            std::thread::sleep(
                std::time::Duration::from_millis(50)
                    .checked_sub(elapsed_time)
                    .unwrap_or_else(|| std::time::Duration::from_millis(0)),
            );
        }
    }

    pub fn act(&mut self) -> Vec<ServerboundPacket> {
        let mut packets = Vec::new();
        if let Some(position) = self.position.as_mut() {
            if self.vertical_speed.abs() < 0.003 {
                self.vertical_speed = 0.0;
            }
            if self.map.is_on_ground(position.x, position.y, position.z) {
                self.vertical_speed = 0.0;
            } else {
                self.vertical_speed -= 0.08;
                self.vertical_speed *= 0.98;
            }

            if let Some(path) = &self.path {
                if let Some(((x, z), jump)) = path.follow((position.x, position.y, position.z), &self.map) {
                    position.x = x;
                    position.z = z;
                    if jump {
                        self.vertical_speed = 0.4;
                    }
                }
            }

            let max_negative_speed = self.map.max_fall(position.x, position.y, position.z);
            if self.vertical_speed < max_negative_speed {
                self.vertical_speed = max_negative_speed;
            }
            position.y += self.vertical_speed;
            packets.push(ServerboundPacket::PlayerPosition {
                x: position.x,
                y: position.y,
                z: position.z,
                on_ground: self.map.is_on_ground(position.x, position.y, position.z),
            });
        }

        packets
    }

    pub fn update(&mut self, packet: ClientboundPacket) -> Vec<ServerboundPacket> {
        let mut responses = Vec::new();
        match packet {
            ClientboundPacket::KeepAlive { keep_alive_id } => {
                responses.push(ServerboundPacket::KeepAlive { keep_alive_id });
            }
            ClientboundPacket::ChunkData { value } => {
                self.map.load_chunk(value);
            }
            ClientboundPacket::UnloadChunk { chunk_x, chunk_y } => {
                self.map.unload_chunk(chunk_x, chunk_y);
            }
            ClientboundPacket::PlayerPositionAndLook {
                mut x,
                mut y,
                mut z,
                mut yaw,
                mut pitch,
                flags,
                teleport_id,
            } => {
                if let Some(old_position) = &self.position {
                    if flags & 0x1 != 0 {
                        x += old_position.x;
                    }
                    if flags & 0x2 != 0 {
                        y += old_position.y;
                    }
                    if flags & 0x4 != 0 {
                        z += old_position.z;
                    }
                    if flags & 0x8 != 0 {
                        pitch = old_position.pitch;
                    }
                    if flags & 0x10 != 0 {
                        yaw = old_position.yaw;
                    }
                };
                self.position = Some(PlayerPosition { x, y, z, yaw, pitch });
                self.vertical_speed = 0.0;
                warn!("Bot teleported at {:?}", self.position);
                responses.push(ServerboundPacket::TeleportConfirm { teleport_id });
                responses.push(ServerboundPacket::PlayerPositionAndRotation {
                    x,
                    y,
                    z,
                    yaw,
                    pitch,
                    on_ground: true,
                });
            }
            ClientboundPacket::SpawnPosition { location } => {
                debug!("Spawn position set to {:?}", location);
                self.spawn_position = Some(location);
            }
            ClientboundPacket::JoinGame { player_id, world_name, .. } => {
                info!("Joined a world! ({})", world_name);
                self.self_entity_id = Some(player_id);
                self.world_name = Some(world_name.to_string());
            }
            ClientboundPacket::UpdateHealth {
                health,
                food,
                food_saturation,
            } => {
                self.health = health;
                self.food = std::cmp::max(food.0, 0) as u32;
                self.food_saturation = food_saturation;

                if health <= 0.0 {
                    info!("Bot died: respawning...");
                    self.vertical_speed = 0.0;
                    responses.push(ServerboundPacket::ClientStatus {
                        action: minecraft_format::game_state::ClientStatus::PerformRespawn,
                    });
                }
            }
            ClientboundPacket::MultiBlockChange {
                value:
                    MultiBlockChange {
                        chunk_section_position,
                        inverse_trust_edges: _,
                        blocks,
                    },
            } => {
                let (chunk_x, chunk_y, chunk_z) = MultiBlockChange::decode_chunk_section_position(chunk_section_position);
                trace!("ClientboundPacket::MultiBlockChange => Setting {} blocks", blocks.items.len());
                for block in blocks.items {
                    let (block, block_x, block_y, block_z) = MultiBlockChange::decode_block(unsafe { std::mem::transmute(block.0) });
                    self.map.set_block_state(chunk_x, chunk_y, chunk_z, block_x, block_y, block_z, block);
                }
            }
            ClientboundPacket::BlockChange { location, block_state } => {
                let chunk_x = location.x.div_euclid(16);
                let chunk_y = location.y.div_euclid(16) as i32;
                let chunk_z = location.z.div_euclid(16);
                let block_x = location.x.rem_euclid(16) as u8;
                let block_y = location.y.rem_euclid(16) as u8;
                let block_z = location.z.rem_euclid(16) as u8;
                trace!("ClientboundPacket::BlockChange => Setting 1 block at {:?}", location);
                self.map.set_block_state(chunk_x, chunk_y, chunk_z, block_x, block_y, block_z, unsafe {
                    std::mem::transmute(block_state.0)
                });
            }
            ClientboundPacket::ChatMessage { message, position, sender } => {
                if message.contains("test_path") {
                    let position = self.position.as_ref().unwrap();
                    let result = self.map.find_path(
                        (position.x.floor() as i32, position.y.floor() as i32, position.z.floor() as i32),
                        (-95, 89, 91),
                    );
                    debug!("path: {:?}", result);
                    self.path = result;
                }
            }
            _ => (),
        }
        responses
    }
}
