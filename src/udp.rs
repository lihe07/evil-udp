use log::error;
use pnet::{packet::udp, transport};
use std::{net::IpAddr, process::exit};

pub fn prepare_sender() -> transport::TransportSender {
    match transport::transport_channel(
        65535,
        transport::TransportChannelType::Layer4(transport::TransportProtocol::Ipv4(
            pnet::packet::ip::IpNextHeaderProtocols::Udp,
        )),
    ) {
        Ok((tx, _)) => tx,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                error!("Permission denied. Try running as root.");
                exit(1);
            }

            error!(
                "An unknown error occurred when creating the transport channel: {}",
                e
            );
            exit(1);
        }
    }
}

pub fn send_raw_pack(
    tx: &mut transport::TransportSender,
    payload: &[u8],
    src_addr: IpAddr,
    src_port: u16,
    dest_addr: IpAddr,
    dest_port: u16,
) -> usize {
    // Use pnet to send raw packets

    let mut udp_buffer = vec![0u8; payload.len() + 8];
    let mut packet = udp::MutableUdpPacket::new(&mut udp_buffer).unwrap();

    // let payload = b"Hello, world!\n";
    packet.set_length(payload.len() as u16 + 8);
    packet.set_source(src_port);
    packet.set_destination(dest_port);
    packet.set_payload(payload);

    let checksum = match (src_addr, dest_addr) {
        (IpAddr::V4(src), IpAddr::V4(dest)) => {
            udp::ipv4_checksum(&packet.to_immutable(), &src, &dest)
        }
        (IpAddr::V6(src), IpAddr::V6(dest)) => {
            udp::ipv6_checksum(&packet.to_immutable(), &src, &dest)
        }

        _ => panic!(),
    };

    packet.set_checksum(checksum);

    tx.send_to(packet, dest_addr).unwrap()
}

pub fn get_ip() -> Option<IpAddr> {
    let interfaces = pnet::datalink::interfaces();
    let interface = interfaces
        .iter()
        .find(|iface| iface.is_up() && !iface.ips.is_empty() && !iface.is_loopback());

    interface.map(|interface| interface.ips[0].ip())
}

#[test]
fn test_send_raw_pack() {
    use std::str::FromStr;
    pretty_env_logger::init();

    let src_addr = IpAddr::from_str("192.168.1.26").unwrap();
    let src_port = 12345;
    let dest_addr = IpAddr::from_str("8.130.162.255").unwrap();
    let dest_port = 15555;

    let mut tx = prepare_sender();
    let payload = b"Hello, world!\n";

    send_raw_pack(&mut tx, payload, src_addr, src_port, dest_addr, dest_port);
}

#[test]
fn test_get_ip() {
    pretty_env_logger::init();
    let ip = get_ip();
    dbg!(ip);
}
