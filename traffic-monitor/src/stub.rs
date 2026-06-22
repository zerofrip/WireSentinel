use shared_types::Protocol;

#[allow(dead_code)]
pub fn parse_protocol(proto: u8) -> Protocol {
    match proto {
        6 => Protocol::Tcp,
        17 => Protocol::Udp,
        1 => Protocol::Icmp,
        _ => Protocol::Other,
    }
}
