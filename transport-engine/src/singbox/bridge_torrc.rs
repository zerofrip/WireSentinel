use shared_types::{BridgeProfile, BridgeType};
use std::collections::HashMap;

/// Extract the tor bridge line from `config_json` (`{ "line": "obfs4 ..." }`).
pub fn bridge_line(profile: &BridgeProfile) -> Option<String> {
    profile
        .config_json
        .get("line")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Build torrc map + extra `--Bridge` args for sing-box tor outbound.
pub fn bridges_to_tor_options(bridges: &[BridgeProfile]) -> (HashMap<String, String>, Vec<String>) {
    let mut torrc = HashMap::new();
    torrc.insert("ClientOnly".into(), "1".into());

    let enabled: Vec<_> = bridges
        .iter()
        .filter(|b| b.enabled)
        .filter_map(bridge_line)
        .collect();

    if enabled.is_empty() {
        return (torrc, Vec::new());
    }

    torrc.insert("UseBridges".into(), "1".into());

    let mut extra_args = Vec::new();
    for line in enabled {
        extra_args.push("--Bridge".into());
        extra_args.push(line);
    }

    (torrc, extra_args)
}

/// Bridge type label for logging (not used in tor line itself).
#[allow(dead_code)]
pub fn bridge_type_label(t: BridgeType) -> &'static str {
    match t {
        BridgeType::Obfs4 => "obfs4",
        BridgeType::Snowflake => "snowflake",
        BridgeType::Meek => "meek",
        BridgeType::Webtunnel => "webtunnel",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    fn sample_bridge(line: &str) -> BridgeProfile {
        BridgeProfile {
            id: Uuid::new_v4(),
            name: "test".into(),
            bridge_type: BridgeType::Obfs4,
            config_json: json!({ "line": line }),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn bridges_add_use_bridges_and_extra_args() {
        let bridges = vec![sample_bridge("obfs4 1.2.3.4:443 cert=abc")];
        let (torrc, extra) = bridges_to_tor_options(&bridges);
        assert_eq!(torrc.get("UseBridges"), Some(&"1".to_string()));
        assert_eq!(torrc.get("ClientOnly"), Some(&"1".to_string()));
        assert_eq!(extra.len(), 2);
        assert_eq!(extra[0], "--Bridge");
        assert!(extra[1].starts_with("obfs4"));
    }

    #[test]
    fn empty_bridges_only_client_only() {
        let (torrc, extra) = bridges_to_tor_options(&[]);
        assert_eq!(torrc.get("ClientOnly"), Some(&"1".to_string()));
        assert!(torrc.get("UseBridges").is_none());
        assert!(extra.is_empty());
    }
}
