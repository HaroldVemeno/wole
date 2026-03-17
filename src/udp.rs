use std::io;
use std::net::UdpSocket;

use pnet::util::MacAddr;

pub fn send(dest_mac: MacAddr) -> io::Result<()> {

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

