use crate::*;
use minecraft_protocol::{components::blocks::MultiBlockChange, components::chat::ChatMode, components::slots::MainHand, MinecraftPacketPart};
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
    pub entities: Entities,
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
        let (receiver, sender) = crate::network::connect(&addr, port, &username);
        info!("{} is connected on {}:{}", username, addr, port);
        let sender2 = sender.clone();

        let bot = Arc::new(Mutex::new(Bot {
            username,
            addr,
            port,
            map: Map::new(),
            entities: Entities::new(sender.clone()),
            position: None,
            spawn_position: None,
            self_entity_id: None,
            world_name: None,
            windows: Windows::new(sender.clone()),
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
                info!("Joined a world! ({}) {}", world_name, player_id);
                self.entities.add_self(player_id);
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
                        action: minecraft_protocol::components::game_state::ClientStatus::PerformRespawn,
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
                //trace!("ClientboundPacket::MultiBlockChange => Setting {} blocks", blocks.items.len());
                for block in blocks.items {
                    let (block, block_x, block_y, block_z) = MultiBlockChange::decode_block(unsafe { std::mem::transmute(block.0) });
                    self.map
                        .set_block_state_complex(chunk_x, chunk_y, chunk_z, block_x, block_y, block_z, block);
                }
            }
            ClientboundPacket::BlockChange { location, block_state } => {
                let chunk_x = location.x.div_euclid(16);
                let chunk_y = location.y.div_euclid(16) as i32;
                let chunk_z = location.z.div_euclid(16);
                let block_x = location.x.rem_euclid(16) as u8;
                let block_y = location.y.rem_euclid(16) as u8;
                let block_z = location.z.rem_euclid(16) as u8;
                //trace!("ClientboundPacket::BlockChange => Setting 1 block at {:?}", location);
                self.map
                    .set_block_state_complex(chunk_x, chunk_y, chunk_z, block_x, block_y, block_z, unsafe {
                        std::mem::transmute(block_state.0)
                    });
            }
            ClientboundPacket::ChatMessage {
                message,
                position: _,
                sender: _,
            } => {
                if message.contains("test path") {
                    let position = self.position.as_ref().unwrap();
                    if let Some(mission) = TravelMission::new(&self.map, (position.x as i32, position.y as i32, position.z as i32), (-222, 75, 54), 7500) {
                        *self.mission.lock().unwrap() = Some(Box::new(mission));
                    }
                } else if message.contains("find diams") {
                    let position = self.position.as_ref().unwrap();
                    info!(
                        "{} diamonds blocks found",
                        self.map
                            .search_blocks(position.x as i32, position.z as i32, &[Block::DiamondBlock, Block::DiamondOre], 5000, 32*32)
                            .len()
                    );
                } else if message.contains("settle") {
                    *self.mission.lock().unwrap() = Some(Box::new(SettlementMission::new()));
                } else if message.contains("dig down") {
                    *self.mission.lock().unwrap() = Some(Box::new(DigDownMission::new(12)));
                } else if message.contains("test inventory 1") {
                    *self.mission.lock().unwrap() = Some(Box::new(MoveItemTo::new(
                        1,
                        vec![minecraft_protocol::ids::items::Item::Sand],
                        35,
                    )));
                } else if message.contains("test inventory 2") {
                    *self.mission.lock().unwrap() = Some(Box::new(MoveItemTo::new(1, vec![minecraft_protocol::ids::items::Item::Sand], 45)));
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
            ClientboundPacket::WindowConfirmation {
                window_id,
                action_id,
                accepted,
            } => {
                self.windows.handle_window_confirmation_packet(window_id, action_id, accepted);
                if !accepted {
                    responses.push(ServerboundPacket::WindowConfirmation {
                        window_id,
                        action_id,
                        accepted,
                    })
                }
            }
            ClientboundPacket::CloseWindow { window_id } => {
                self.windows.handle_close_window_packet(window_id);
            }
            ClientboundPacket::HeldItemChange { slot } => {
                self.windows.player_inventory.handle_held_item_change_packet(slot);
            }
            ClientboundPacket::SpawnEntity {
                id,
                uuid,
                entity_type,
                x,
                y,
                z,
                pitch,
                yaw,
                data,
                velocity_x,
                velocity_y,
                velocity_z,
            } => {
                self.entities
                    .handle_spawn_entity_packet(id.0, uuid, entity_type, x, y, z, pitch, yaw, data, velocity_x, velocity_y, velocity_z);
            }
            ClientboundPacket::EntityMetadata { entity_id, metadata } => {
                self.entities.handle_entity_metadata_packet(entity_id.0, metadata);
            }
            ClientboundPacket::SpawnPlayer {
                id,
                uuid,
                x,
                y,
                z,
                yaw,
                pitch,
            } => {
                self.entities.handle_spawn_player_packet(id.0, uuid, x, y, z, yaw, pitch);
            }
            ClientboundPacket::SpawnLivingEntity {
                id,
                uuid,
                entity_type,
                x,
                y,
                z,
                yaw,
                pitch,
                head_pitch,
                velocity_x,
                velocity_y,
                velocity_z,
            } => {
                self.entities.handle_spawn_living_entity_packet(
                    id.0,
                    uuid,
                    entity_type,
                    x,
                    y,
                    z,
                    yaw,
                    pitch,
                    head_pitch,
                    velocity_x,
                    velocity_y,
                    velocity_z,
                );
            }
            ClientboundPacket::SpawnExperienceOrb { id, x, y, z, count } => {
                self.entities.handle_spawn_experience_orb_packet(id.0, x, y, z, count);
            }
            ClientboundPacket::SpawnPainting {
                id,
                uuid,
                motive,
                location,
                direction,
            } => {
                self.entities.handle_spawn_painting_packet(id.0, uuid, motive, location, direction);
            }
            ClientboundPacket::EntityAnimation { .. } | ClientboundPacket::EntityStatus { .. } | ClientboundPacket::EntityHeadLook { .. } => {
                // Unsupported as it is primarly used in animations
            }
            ClientboundPacket::EntityPosition {
                entity_id,
                delta_x,
                delta_y,
                delta_z,
                on_ground,
            } => {
                self.entities
                    .handle_entity_position_packet(entity_id.0, delta_x, delta_y, delta_z, on_ground);
            }
            ClientboundPacket::EntityPositionAndRotation {
                entity_id,
                delta_x,
                delta_y,
                delta_z,
                yaw,
                pitch,
                on_ground,
            } => {
                self.entities
                    .handle_entity_position_and_rotation_packet(entity_id.0, delta_x, delta_y, delta_z, yaw, pitch, on_ground);
            }
            ClientboundPacket::EntityMovement { entity_id } => {
                self.entities.handle_entity_movement_packet(entity_id.0);
            }
            ClientboundPacket::DestoryEntities { entity_ids } => {
                self.entities
                    .handle_destroy_entities_packet(entity_ids.items.iter().map(|varint| varint.0).collect());
            }
            ClientboundPacket::RemoveEntityEffect { entity_id, effect } => {
                self.entities.handle_remove_entity_effect_packet(entity_id.0, effect);
            }
            ClientboundPacket::EntityVelocity {
                entity_id,
                velocity_x,
                velocity_y,
                velocity_z,
            } => {
                self.entities
                    .handle_entity_velocity_packet(entity_id.0, velocity_x, velocity_y, velocity_z);
            }
            ClientboundPacket::EntityEquipment { entity_id, equipment } => {
                self.entities.handle_entity_equipement_packet(entity_id.0, equipment);
            }
            ClientboundPacket::TeleportEntity {
                entity_id,
                x,
                y,
                z,
                yaw,
                pitch,
                on_ground,
            } => {
                self.entities.handle_teleport_entity_packet(entity_id.0, x, y, z, yaw, pitch, on_ground);
            }
            ClientboundPacket::EntityAttributes { entity_id, attributes } => {
                self.entities.handle_entity_attributes_packet(entity_id.0, attributes);
            }
            _ => (),
        }
        responses
    }
}
