use crate::network::connect;
use log::*;
use minecraft_format::{
    packets::{play_clientbound::ClientboundPacket, play_serverbound::ServerboundPacket},
    MinecraftPacketPart,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct Bot {
    username: String,
    addr: String,
    port: u16,
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
            ClientboundPacket::ChunkData { value: _ } => {
                // TODO
            }
            _ => (),
        }
        responses
    }
}
