use crate::{
    inventory::Windows,
    map::Map,
    missions::*,
    network::connect,
};
use log::*;
use minecraft_format::{
    blocks::MultiBlockChange,
    chat::ChatMode,
    packets::{play_clientbound::ClientboundPacket, play_serverbound::ServerboundPacket, Position},
    slots::MainHand,
    MinecraftPacketPart,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug)]
pub struct PlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

pub struct Bot {
    pub username: String,
    pub addr: String,
    pub port: u16,
    pub map: Map,
    pub self_entity_id: Option<i32>,
    pub position: Option<PlayerPosition>,
    pub spawn_position: Option<Position>,
    pub world_name: Option<String>,
    pub windows: Windows,

    pub health: f32,
    pub food: u32,
    pub food_saturation: f32,
    pub vertical_speed: f64,
    pub mission: Arc<Mutex<Option<Box<dyn Mission>>>>,
}

impl Bot {
    pub fn create(addr: String, port: u16, username: String) {
        debug!("Connecting {} to {}:{}", username, addr, port);
        let (receiver, sender) = connect(&addr, port, &username);
        info!("{} is connected on {}:{}", username, addr, port);
        let sender2 = sender.clone();
        let sender3 = sender.clone();

        let bot = Arc::new(Mutex::new(Bot {
            username,
            addr,
            port,
            map: Map::new(),
            position: None,
            spawn_position: None,
            self_entity_id: None,
            world_name: None,
            windows: Windows::new(sender3),
            mission: Arc::new(Mutex::new(None)),

            health: 11.0,
            food: 11,
            food_saturation: 0.0,
            vertical_speed: 0.0,
        }));
        let bot2 = Arc::clone(&bot);

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
        }

        // TODO, replace path with mission

        let mission = Arc::clone(&self.mission);
        if let Some(mission) = mission.lock().unwrap().as_mut() {
            mission.execute(self, &mut packets);
        }

        if let Some(position) = self.position.as_mut() {
            let max_negative_speed = self.map.max_fall(position.x, position.y, position.z);
            //trace!("{} {} {} {}", self.map.is_on_ground(position.x, position.y, position.z), max_negative_speed, self.vertical_speed, position.y);
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
            ClientboundPacket::ChatMessage { message, position: _, sender: _ } => {
                if message.contains("test path") {
                    let position = self.position.as_ref().unwrap();
                    let position = (position.x.floor() as i32, position.y.floor() as i32, position.z.floor() as i32);
                    if let Some(travel_mission) = TravelMission::new((-77, 89, 88), &self.map, position) {
                        *self.mission.lock().unwrap() = Some(Box::new(travel_mission));
                    }
                } else if message.contains("dig down") {
                    *self.mission.lock().unwrap() = Some(Box::new(DigDownMission::new(12)));
                } else if message.contains("test inventory 1") {
                    *self.mission.lock().unwrap() = Some(Box::new(MoveItemToHotbar::new(1, vec![minecraft_format::ids::items::Item::Sand], Some(3))));
                } else if message.contains("test inventory 2") {
                    *self.mission.lock().unwrap() = Some(Box::new(MoveItemToHotbar::new(1, vec![minecraft_format::ids::items::Item::Sand], None)));
                }
            }
            ClientboundPacket::OpenWindow {
                window_id,
                window_type,
                window_title: _,
            } => {
                self.windows.handle_open_window_packet(window_id.0, window_type.0);
            }
            ClientboundPacket::WindowItems { window_id, slots } => {
                self.windows.handle_update_window_items_packet(window_id, slots);
            }
            ClientboundPacket::SetSlot {
                window_id,
                slot_index,
                slot_value,
            } => {
                self.windows.handle_set_slot_packet(window_id, slot_index, slot_value);
            }
            ClientboundPacket::WindowConfirmation { window_id, action_id, accepted } => {
                self.windows.handle_window_confirmation_packet(window_id, action_id, accepted);
                if !accepted {
                    responses.push(ServerboundPacket::WindowConfirmation{window_id, action_id, accepted})
                }
            }
            ClientboundPacket::CloseWindow { window_id } => {
                self.windows.handle_close_window_packet(window_id);
            }
            _ => (),
        }
        responses
    }
}
