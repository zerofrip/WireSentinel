use std::collections::HashSet;

/// Parse EasyList-style domain rules (`||domain^`).
pub fn parse(content: &str) -> HashSet<String> {
    let mut domains = HashSet::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
            continue;
        }
        if let Some(domain) = parse_rule(line) {
            domains.insert(domain);
        }
    }
    domains
}

fn parse_rule(line: &str) -> Option<String> {
    let rest = line.strip_prefix("||")?;
    let end = rest
        .find(['^', '/', '*'])
        .unwrap_or(rest.len());
    let domain = normalize_domain(&rest[..end]);
    if domain.is_empty() {
        None
    } else {
        Some(domain)
    }
}

fn normalize_domain(domain: &str) -> String {
    domain.trim_end_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_domain_rules() {
        let input = "! comment\n||ads.example.com^\n||tracker.test/path\n";
        let set = parse(input);
        assert!(set.contains("ads.example.com"));
        assert!(set.contains("tracker.test"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn ignores_non_domain_rules() {
        let input = "example.com##.banner\n";
        let set = parse(input);
        assert!(set.is_empty());
    }
}
