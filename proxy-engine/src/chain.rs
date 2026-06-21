//! Proxy chain hop ordering and compatibility validation.

use shared_types::{ProxyChainHop, ProxyChainHopKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainValidationError {
    pub message: String,
}

impl std::fmt::Display for ChainValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChainValidationError {}

pub fn validate_hop_sequence(hops: &[ProxyChainHop]) -> Result<(), ChainValidationError> {
    if hops.is_empty() {
        return Err(ChainValidationError {
            message: "proxy chain requires at least one hop".into(),
        });
    }

    let mut ordered = hops.to_vec();
    ordered.sort_by_key(|h| h.order);

    for window in ordered.windows(2) {
        let from = window[0].kind;
        let to = window[1].kind;
        if !is_valid_transition(from, to) {
            return Err(ChainValidationError {
                message: format!("invalid hop transition: {from:?} -> {to:?}"),
            });
        }
    }

    Ok(())
}

fn is_valid_transition(from: ProxyChainHopKind, to: ProxyChainHopKind) -> bool {
    use ProxyChainHopKind::*;
    match (from, to) {
        (Https, Socks5) | (Https, Http) => false,
        (Tor, Tor) => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn hop(kind: ProxyChainHopKind, order: u32) -> ProxyChainHop {
        ProxyChainHop {
            kind,
            profile_id: Uuid::new_v4(),
            order,
        }
    }

    #[test]
    fn socks5_to_https_is_valid() {
        let hops = vec![
            hop(ProxyChainHopKind::Socks5, 0),
            hop(ProxyChainHopKind::Https, 1),
        ];
        assert!(validate_hop_sequence(&hops).is_ok());
    }

    #[test]
    fn https_to_socks5_is_invalid() {
        let hops = vec![
            hop(ProxyChainHopKind::Https, 0),
            hop(ProxyChainHopKind::Socks5, 1),
        ];
        assert!(validate_hop_sequence(&hops).is_err());
    }

    #[test]
    fn empty_chain_is_invalid() {
        assert!(validate_hop_sequence(&[]).is_err());
    }
}
