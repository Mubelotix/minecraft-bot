use minecraft_format::packets::{
    play_clientbound::ClientboundPacket, play_serverbound::ServerboundPacket,
};
use minecraft_format::*;

pub mod network;
use network::*;

fn main() {
    let (receiver, sender) = connect();

    loop {
        let mut packet_bytes = receiver.recv().unwrap();
        let packet =
            ClientboundPacket::deserialize_uncompressed_minecraft_packet(&mut packet_bytes);
        let packet = match packet {
            Ok(packet) => packet,
            Err(e) => panic!("{} for {:?}", e, packet_bytes),
        };
        match packet {
            ClientboundPacket::KeepAlive { keep_alive_id } => {
                sender.send(
                    ServerboundPacket::KeepAlive { keep_alive_id }.serialize_minecraft_packet().unwrap(),
                )
                .unwrap();
                println!("pong!");
            }
            ClientboundPacket::ChunkData { mut value } => {
                value.deserialize_chunk_sections().unwrap();
                println!("chunk parsed successfully!")
            }
            ClientboundPacket::ChatMessage {
                message,
                position: _,
                sender,
            } => {
                println!("{}: {}", sender, message);
            }
            _ => (),
        }
    }
}
