use crate::{map::Map, network::connect};
use log::*;
use minecraft_format::{
    gamemode::Gamemode,
    packets::{play_clientbound::ClientboundPacket, play_serverbound::ServerboundPacket, Position},
    MinecraftPacketPart,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
}

impl Bot {
    pub fn create(addr: String, port: u16, username: String) {
        debug!("Connecting {} to {}:{}", addr, port, username);
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
        }));
        let bot2 = Arc::clone(&bot);
        let sender2 = sender.clone();

        std::thread::spawn(move || loop {
            let mut packet = receiver.recv().unwrap();
            let packet = match ClientboundPacket::deserialize_uncompressed_minecraft_packet(
                packet.as_mut_slice(),
            ) {
                Ok(packet) => packet,
                Err(e) => {
                    eprintln!("Failed to parse clientbound packet: {}", e);
                    continue;
                }
            };
            let mut bot = bot.lock().unwrap();
            let response_packets = bot.update(packet);
            for response_packet in response_packets {
                let response_packet = match response_packet.serialize_minecraft_packet() {
                    Ok(response_packet) => response_packet,
                    Err(e) => {
                        eprintln!("Failed to serialize packet from client {}", e);
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
                            eprintln!("Failed to serialize packet from client {}", e);
                            continue;
                        }
                    };
                    sender2.send(response_packet).unwrap();
                }
            }
            let elapsed_time = Instant::now() - start_time;
            std::thread::sleep(std::time::Duration::from_millis(50) - elapsed_time);
        }
    }

    pub fn act(&mut self) -> Vec<ServerboundPacket> {
        Vec::new()
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
                self.position = Some(PlayerPosition {
                    x,
                    y,
                    z,
                    yaw,
                    pitch,
                });
                info!("Bot teleported at {:?}", self.position);
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
            ClientboundPacket::JoinGame {
                player_id,
                world_name,
                ..
            } => {
                info!("Joined a world! ({})", world_name);
                self.self_entity_id = Some(player_id);
                self.world_name = Some(world_name.to_string());
            }
            _ => (),
        }
        responses
    }
}
