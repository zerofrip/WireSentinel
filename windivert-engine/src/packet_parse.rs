//! Minimal IPv4/IPv6 + TCP/UDP header parsing for WinDivert capture.

use shared_types::Protocol;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowEndpoints {
    pub protocol: Protocol,
    pub local: SocketAddr,
    pub remote: SocketAddr,
    pub outbound: bool,
}

/// Parse network-layer packet bytes into flow endpoints.
pub fn parse_packet(raw: &[u8], outbound: bool, ipv6_hint: bool) -> Option<FlowEndpoints> {
    if raw.is_empty() {
        return None;
    }
    let version = raw[0] >> 4;
    match version {
        4 if !ipv6_hint => parse_ipv4(raw, outbound),
        6 => parse_ipv6(raw, outbound),
        4 => parse_ipv4(raw, outbound),
        _ => None,
    }
}

fn parse_ipv4(raw: &[u8], outbound: bool) -> Option<FlowEndpoints> {
    if raw.len() < 20 {
        return None;
    }
    let ihl = (raw[0] & 0x0f) as usize * 4;
    if ihl < 20 || raw.len() < ihl {
        return None;
    }
    let proto = raw[9];
    let src = Ipv4Addr::new(raw[12], raw[13], raw[14], raw[15]);
    let dst = Ipv4Addr::new(raw[16], raw[17], raw[18], raw[19]);
    parse_transport(
        proto,
        &raw[ihl..],
        IpAddr::V4(src),
        IpAddr::V4(dst),
        outbound,
    )
}

fn parse_ipv6(raw: &[u8], outbound: bool) -> Option<FlowEndpoints> {
    if raw.len() < 40 {
        return None;
    }
    let mut next_header = raw[6];
    let mut offset = 40usize;
    let src = Ipv6Addr::from([
        raw[8], raw[9], raw[10], raw[11], raw[12], raw[13], raw[14], raw[15], raw[16], raw[17],
        raw[18], raw[19], raw[20], raw[21], raw[22], raw[23],
    ]);
    let dst = Ipv6Addr::from([
        raw[24], raw[25], raw[26], raw[27], raw[28], raw[29], raw[30], raw[31], raw[32], raw[33],
        raw[34], raw[35], raw[36], raw[37], raw[38], raw[39],
    ]);
    while offset + 8 <= raw.len() && (next_header == 0 || next_header == 43 || next_header == 44) {
        next_header = raw[offset];
        let ext_len = raw[offset + 1] as usize;
        offset += (ext_len + 1) * 8;
    }
    if offset >= raw.len() {
        return None;
    }
    parse_transport(
        next_header,
        &raw[offset..],
        IpAddr::V6(src),
        IpAddr::V6(dst),
        outbound,
    )
}

fn parse_transport(
    proto: u8,
    payload: &[u8],
    src_ip: IpAddr,
    dst_ip: IpAddr,
    outbound: bool,
) -> Option<FlowEndpoints> {
    let protocol = match proto {
        6 => Protocol::Tcp,
        17 => Protocol::Udp,
        _ => return None,
    };
    if payload.len() < 4 {
        return None;
    }
    let src_port = u16::from_be_bytes([payload[0], payload[1]]);
    let dst_port = u16::from_be_bytes([payload[2], payload[3]]);
    let (local, remote) = if outbound {
        (
            SocketAddr::new(src_ip, src_port),
            SocketAddr::new(dst_ip, dst_port),
        )
    } else {
        (
            SocketAddr::new(dst_ip, dst_port),
            SocketAddr::new(src_ip, src_port),
        )
    };
    Some(FlowEndpoints {
        protocol,
        local,
        remote,
        outbound,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn parse_ipv4_tcp_syn() {
        // IPv4 TCP SYN 192.168.1.2:45678 -> 93.184.216.34:443
        let mut pkt = vec![
            0x45, 0x00, 0x00, 0x28, 0x00, 0x00, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 0xc0, 0xa8,
            0x01, 0x02, 0x5d, 0xb8, 0xd8, 0x22, 0xb2, 0x6e, 0x01, 0xbb, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x50, 0x02, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
        ];
        let _ = &mut pkt;
        let flow = parse_packet(&pkt, true, false).expect("parse");
        assert_eq!(flow.protocol, Protocol::Tcp);
        assert_eq!(
            flow.local,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), 45678)
        );
        assert_eq!(
            flow.remote,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)), 443)
        );
    }
}
