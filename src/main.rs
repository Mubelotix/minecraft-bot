use minecraft_format::packets::{
    play_clientbound::ClientboundPacket, play_serverbound::ServerboundPacket,
};
use minecraft_format::{network::*, *};
use std::{net::TcpStream, sync::mpsc};

fn receive_packets(hidden_sender: mpsc::Sender<Vec<u8>>, stream: TcpStream) {
    loop {
        let packet = read_packet(&stream, None, None).unwrap();
        hidden_sender.send(packet).unwrap();
    }
}

fn send_packets(hidden_receiver: mpsc::Receiver<Vec<u8>>, stream: TcpStream) {
    loop {
        let packet = hidden_receiver.recv().unwrap();
        send_packet(&stream, packet, None, None).unwrap();
    }
}

fn connect() -> (mpsc::Receiver<Vec<u8>>, mpsc::Sender<Vec<u8>>) {
    let mut stream = TcpStream::connect("127.0.0.1:25565").unwrap();
    send_packet(
        &mut stream,
        minecraft_format::packets::handshake::ServerboundPacket::Hello {
            protocol_version: 754.into(),
            server_address: "127.0.0.1",
            server_port: 25565,
            next_state: minecraft_format::packets::ConnectionState::Login,
        }.serialize_minecraft_packet().unwrap(),
        None,
        None,
    )
    .unwrap();

    send_packet(
        &mut stream,
        minecraft_format::packets::login::ServerboundPacket::LoginStart { username: "bot2" }.serialize_minecraft_packet().unwrap(),
        None,
        None,
    )
    .unwrap();

    let mut response = read_packet(&stream, None, None).unwrap();
    let response_packet =
        minecraft_format::packets::login::ClientboundPacket::deserialize_uncompressed_minecraft_packet(
            &mut response,
        )
        .unwrap();
    println!("{:?}", response_packet);
    
    let stream2 = stream.try_clone().unwrap();

    let (hidden_sender, receiver) = mpsc::channel::<Vec<u8>>();
    let (sender, hidden_receiver) = mpsc::channel::<Vec<u8>>();

    std::thread::spawn(|| {
        receive_packets(hidden_sender, stream);
    });
    std::thread::spawn(|| {
        send_packets(hidden_receiver, stream2);
    });

    (receiver, sender)
}

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
