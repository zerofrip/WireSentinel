use proxy_engine::validate_hop_sequence;
use shared_types::{ProxyChainHop, ProxyChainHopKind};
use uuid::Uuid;

fn hop(kind: ProxyChainHopKind, order: u32) -> ProxyChainHop {
    ProxyChainHop {
        kind,
        profile_id: Uuid::new_v4(),
        order,
    }
}

#[test]
fn validates_ordered_hops() {
    let hops = vec![
        hop(ProxyChainHopKind::Socks5, 0),
        hop(ProxyChainHopKind::Http, 1),
        hop(ProxyChainHopKind::Https, 2),
    ];
    assert!(validate_hop_sequence(&hops).is_ok());
}

#[test]
fn rejects_https_followed_by_http() {
    let hops = vec![hop(ProxyChainHopKind::Https, 0), hop(ProxyChainHopKind::Http, 1)];
    assert!(validate_hop_sequence(&hops).is_err());
}
