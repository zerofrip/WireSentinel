use filter_lists::FilterListProvider;
use shared_types::DnsFilterMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainDecision {
    Allow,
    Block,
}

pub fn evaluate_domain(
    domain: &str,
    filter_mode: DnsFilterMode,
    provider: Option<&dyn FilterListProvider>,
) -> DomainDecision {
    let Some(provider) = provider else {
        return DomainDecision::Allow;
    };

    let listed = provider.is_blocked(domain);
    match filter_mode {
        DnsFilterMode::Blacklist => {
            if listed {
                DomainDecision::Block
            } else {
                DomainDecision::Allow
            }
        }
        DnsFilterMode::Whitelist => {
            if listed {
                DomainDecision::Allow
            } else {
                DomainDecision::Block
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        domains: Vec<String>,
    }

    impl MockProvider {
        fn new(domains: &[&str]) -> Self {
            Self {
                domains: domains.iter().map(|d| d.to_string()).collect(),
            }
        }
    }

    #[async_trait::async_trait]
    impl FilterListProvider for MockProvider {
        async fn update_all(&self) -> Result<(), String> {
            Ok(())
        }

        fn is_blocked(&self, domain: &str) -> bool {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            self.domains.iter().any(|d| d == &domain)
        }
    }

    #[test]
    fn blacklist_blocks_listed() {
        let provider = MockProvider::new(&["ads.example.com"]);
        assert_eq!(
            evaluate_domain("ads.example.com", DnsFilterMode::Blacklist, Some(&provider),),
            DomainDecision::Block
        );
        assert_eq!(
            evaluate_domain(
                "safe.example.com",
                DnsFilterMode::Blacklist,
                Some(&provider)
            ),
            DomainDecision::Allow
        );
    }

    #[test]
    fn whitelist_allows_listed_only() {
        let provider = MockProvider::new(&["allowed.example.com"]);
        assert_eq!(
            evaluate_domain(
                "allowed.example.com",
                DnsFilterMode::Whitelist,
                Some(&provider),
            ),
            DomainDecision::Allow
        );
        assert_eq!(
            evaluate_domain(
                "other.example.com",
                DnsFilterMode::Whitelist,
                Some(&provider),
            ),
            DomainDecision::Block
        );
    }

    #[test]
    fn no_provider_allows() {
        assert_eq!(
            evaluate_domain("any.test", DnsFilterMode::Blacklist, None),
            DomainDecision::Allow
        );
    }
}
