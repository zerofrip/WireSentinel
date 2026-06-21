use std::collections::HashSet;

/// Parse a hosts-format filter list (`0.0.0.0 domain` lines).
pub fn parse(content: &str) -> HashSet<String> {
    let mut domains = HashSet::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(ip) = parts.next() else {
            continue;
        };
        if !is_block_ip(ip) {
            continue;
        }

        for domain in parts {
            if domain.starts_with('#') {
                break;
            }
            let domain = normalize_domain(domain);
            if !domain.is_empty() && domain != "localhost" {
                domains.insert(domain);
            }
        }
    }
    domains
}

fn is_block_ip(ip: &str) -> bool {
    matches!(
        ip,
        "0.0.0.0"
            | "127.0.0.1"
            | "::"
            | "0:0:0:0:0:0:0:0"
            | "::1"
            | "0000:0000:0000:0000:0000:0000:0000:0000"
    )
}

fn normalize_domain(domain: &str) -> String {
    domain.trim_end_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_block_entries() {
        let input = "# comment\n0.0.0.0 ads.example.com\n127.0.0.1 tracker.test\n";
        let set = parse(input);
        assert!(set.contains("ads.example.com"));
        assert!(set.contains("tracker.test"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn skips_non_block_ips() {
        let input = "192.168.1.1 internal.local\n";
        let set = parse(input);
        assert!(set.is_empty());
    }
}
