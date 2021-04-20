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
        log::trace!("Sending {:?}", packet);
        send_packet(&stream, packet, None, None).unwrap();
    }
}

pub fn connect(addr: &str, port: u16, username: &str) -> (mpsc::Receiver<Vec<u8>>, mpsc::Sender<Vec<u8>>) {
    let mut stream = TcpStream::connect(format!("{}:{}", addr, port)).unwrap();
    send_packet(
        &mut stream,
        minecraft_format::packets::handshake::ServerboundPacket::Hello {
            protocol_version: 754.into(),
            server_address: addr,
            server_port: port,
            next_state: minecraft_format::packets::ConnectionState::Login,
        }
        .serialize_minecraft_packet()
        .unwrap(),
        None,
        None,
    )
    .unwrap();

    send_packet(
        &mut stream,
        minecraft_format::packets::login::ServerboundPacket::LoginStart { username }
            .serialize_minecraft_packet()
            .unwrap(),
        None,
        None,
    )
    .unwrap();

    let response = read_packet(&stream, None, None).unwrap();
    let response_packet = minecraft_format::packets::login::ClientboundPacket::deserialize_uncompressed_minecraft_packet(&response).unwrap();
    assert!(matches!(
        response_packet,
        minecraft_format::packets::login::ClientboundPacket::LoginSuccess { .. }
    ));

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
