use std::io;
use std::net::UdpSocket;

use clap::Parser;

use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{checksum, Ipv4, Ipv4Flags, MutableIpv4Packet};
use pnet::packet::ethernet::{EtherTypes, Ethernet, MutableEthernetPacket};
use pnet::ipnetwork::{IpNetwork, Ipv4Network};
use pnet::packet::Packet;
use pnet::util::MacAddr;



#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {

    #[arg(short, long)]
    save: bool,

    #[arg(short, long)]
    raw: bool,

    #[arg(short, long)]
    if_name: Option<String>,

    mac_string: String
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let dest_mac = args.mac_string.parse().unwrap();
    if args.raw {
        if let Some(if_name) = args.if_name {
            let interface = datalink::interfaces()
                                     .into_iter()
                                     .filter(|ifc: &NetworkInterface| ifc.name == if_name)
                                     .next().unwrap();
            wol_raw(&interface, dest_mac)?;
        } else {
            for interface in datalink::interfaces() {
                if !interface.is_loopback()
                  && interface.is_up()
                  && interface.mac.is_some() {
                    println!("Trying interface {}", interface.name);
                    wol_raw(&interface, dest_mac)?;
                }
            }
        }
    } else {
        wol_udp(dest_mac)?;
    }

    Ok(())
}

fn wol_udp(dest_mac: MacAddr) -> io::Result<()> {

    let magic: Vec<MacAddr> =  [MacAddr::broadcast()].into_iter()
                               .chain([dest_mac].into_iter().cycle().take(16)).collect();
    let magic_data: Vec<u8> = magic.into_iter().flat_map(|mac| mac.octets()).collect();

    let sock = UdpSocket::bind("0.0.0.0:0")?;
    // let sock = UdpSocket::bind("[::]:0").expect("Unable to bind");
    sock.set_broadcast(true).expect("Unable to enable broadcast on socket");
    sock.send_to(magic_data.as_slice(), "255.255.255.255:40000")?;
    // sock.send_to(magic_data.as_slice(), "[ff02::1%2]:40000").expect("Unable to send packet ");

    Ok(())
}

fn wol_raw(interface: &NetworkInterface, dest_mac: MacAddr) -> io::Result<()> {
    let if_ipnet: Ipv4Network = interface.ips.iter()
                            .filter_map(|&ipn|
                                match ipn {
                                    IpNetwork::V4(ip4_net) => Some(ip4_net),
                                    _ => None
                                }).next().unwrap();
    let (mut tx, _) = match datalink::channel(&interface, Default::default()) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
    };

    let magic : Vec<MacAddr> =        [MacAddr::broadcast()].into_iter()
                               .chain([dest_mac].into_iter().cycle().take(16)).collect();

    let mut packet = Ipv4 {
        version: 4,
        header_length: 5,
        dscp: 0,
        ecn: 0,
        total_length: 0, // will be set correctly
        identification: 0,
        flags: Ipv4Flags::DontFragment,
        fragment_offset: 0,
        ttl: 15,
        next_level_protocol: IpNextHeaderProtocols::ZeroHop,
        checksum: 0, // will be set correctly
        source: if_ipnet.ip(),
        destination: if_ipnet.broadcast(),
        options: vec![],
        payload: magic.into_iter().flat_map(|mac| mac.octets()).collect(),
    };

    packet.total_length = MutableIpv4Packet::packet_size(&packet) as u16;

    let frame = Ethernet{
        source: interface.mac.unwrap(),
        destination: dest_mac,
        ethertype: EtherTypes::WakeOnLan,
        payload: {
            let mut wire_packet = MutableIpv4Packet::owned(vec![0; packet.total_length as usize]).unwrap();
            wire_packet.populate(&packet);
            wire_packet.set_checksum(checksum(&wire_packet.to_immutable()));
            wire_packet.packet().to_vec()
        }
    };

    let mut wire_frame = MutableEthernetPacket::owned(vec![0; MutableEthernetPacket::packet_size(&frame)]).unwrap();
    wire_frame.populate(&frame);

    tx.send_to(wire_frame.packet(), None).unwrap()?;

    Ok(())
}
